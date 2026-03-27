#!/bin/bash
# ============================================================
# CPU 격리 및 커널 튜닝 스크립트
# 기술문서 §5.2: "리눅스 커널 튜닝(isolcpus)을 통해
# 트레이딩 스레드가 OS 인터럽트의 방해를 받지 않도록 설정"
# ============================================================

set -euo pipefail

# 색상 정의
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 로깅 함수
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# ============================================================
# 1. 시스템 정보 수집
# ============================================================
log_info "시스템 정보 수집 중..."

# CPU 코어 개수
TOTAL_CPUS=$(nproc)
log_info "전체 CPU 코어: $TOTAL_CPUS"

# NUMA 노드 확인
if command -v numactl &> /dev/null; then
    NUMA_NODES=$(numactl --hardware | grep "available:" | awk '{print $2}')
    log_info "NUMA 노드 개수: $NUMA_NODES"
else
    log_warn "numactl이 설치되지 않았습니다."
    NUMA_NODES=1
fi

# ============================================================
# 2. 격리할 CPU 코어 결정
# ============================================================
# 트레이딩 스레드용으로 마지막 4개 코어를 격리
# (시스템은 처음 N-4개 코어 사용)
ISOLATED_CPUS=$((TOTAL_CPUS - 4))"-"$((TOTAL_CPUS - 1))
log_info "격리할 CPU 코어: $ISOLATED_CPUS"
log_info "시스템용 CPU 코어: 0-$((TOTAL_CPUS - 5))"

# ============================================================
# 3. GRUB 부트로더 설정 (isolcpus)
# ============================================================
# 참고: isolcpus는 커널 부트 파라미터로 설정되어야 하므로
# GRUB 설정을 수정하고 재부팅해야 합니다.

log_info "GRUB 설정 점검..."

GRUB_CONFIG="/etc/default/grub"

if [ -f "$GRUB_CONFIG" ]; then
    # 현재 GRUB_CMDLINE_LINUX 확인
    if grep -q "isolcpus" "$GRUB_CONFIG"; then
        log_warn "isolcpus 파라미터가 이미 설정되어 있습니다."
    else
        log_warn "GRUB 파라미터 수정이 필요합니다."
        log_warn "다음 명령을 수동으로 실행하세요:"
        log_warn "sudo sed -i \"s/GRUB_CMDLINE_LINUX=\\\"/GRUB_CMDLINE_LINUX=\\\"isolcpus=$ISOLATED_CPUS rcu_nocbs=$ISOLATED_CPUS nohz_full=$ISOLATED_CPUS /\" $GRUB_CONFIG"
        log_warn "sudo grub-mkconfig -o /boot/grub/grub.cfg"
        log_warn "sudo reboot"
    fi
else
    log_error "GRUB 설정 파일을 찾을 수 없습니다: $GRUB_CONFIG"
fi

# ============================================================
# 4. IRQ 어피니티 설정
# ============================================================
# 시스템 인터럽트를 시스템용 CPU로만 라우팅
# 격리된 CPU는 인터럽트를 받지 않도록 설정

log_info "IRQ 어피니티 설정..."

SYSTEM_CPUS="0-$((TOTAL_CPUS - 5))"

# 일반 IRQ 어피니티 설정
if [ -d "/proc/irq" ]; then
    for irq_dir in /proc/irq/*/; do
        irq_num=$(basename "$irq_dir")

        # 시스템 IRQ 스킵 (timer, scheduler 등)
        if [[ "$irq_num" =~ ^[0-9]+$ ]]; then
            # 어피니티 설정 (시스템 CPU만)
            if [ -f "$irq_dir/smp_affinity_list" ]; then
                # smp_affinity_list가 있으면 범위 형식 사용
                echo "$SYSTEM_CPUS" > "$irq_dir/smp_affinity_list" 2>/dev/null || true
            elif [ -f "$irq_dir/smp_affinity" ]; then
                # 16진수 형식으로 변환 (비트마스크)
                # 이 부분은 간단화되었으므로 실제 구현에서는 더 복잡합니다.
                echo "f" > "$irq_dir/smp_affinity" 2>/dev/null || true
            fi
        fi
    done
    log_info "IRQ 어피니티 설정 완료"
else
    log_warn "/proc/irq 디렉토리를 찾을 수 없습니다."
fi

# ============================================================
# 5. NUMA 노드 바인딩
# ============================================================
# NUMA 시스템인 경우 트레이딩 스레드를 특정 NUMA 노드에 바인딩

if [ "$NUMA_NODES" -gt 1 ]; then
    log_info "NUMA 노드 바인딩 설정..."

    # 마지막 NUMA 노드에 격리된 CPU와 메모리 바인딩
    LAST_NODE=$((NUMA_NODES - 1))
    log_info "트레이딩 스레드를 NUMA 노드 $LAST_NODE에 바인딩"

    # numastat 명령 참고:
    # numastat -n  (노드별 메모리 통계)
    # numactl -H   (NUMA 노드 정보)
else
    log_info "NUMA 시스템이 아니므로 바인딩 스킵"
fi

# ============================================================
# 6. Transparent Huge Pages (THP) 비활성화
# ============================================================
# THP는 페이지 스왑으로 인한 지연을 유발할 수 있으므로 비활성화

log_info "Transparent Huge Pages 설정..."

THP_PATH="/sys/kernel/mm/transparent_hugepage/enabled"

if [ -f "$THP_PATH" ]; then
    CURRENT_THP=$(cat "$THP_PATH")

    # THP 비활성화 (madvise만 허용)
    # "never" - 모두 비활성화
    # "madvise" - 명시적 요청시만
    # "always" - 항상 사용

    if grep -q "always" <<< "$CURRENT_THP"; then
        log_info "THP를 'madvise'로 변경 중..."
        echo "madvise" > "$THP_PATH" 2>/dev/null || log_warn "THP 변경 권한이 없습니다."
    fi

    log_info "현재 THP 상태: $(cat $THP_PATH)"
else
    log_warn "THP 설정 파일을 찾을 수 없습니다."
fi

# ============================================================
# 7. 트레이딩 스레드 CPU 고정
# ============================================================
# taskset을 사용하여 특정 프로세스를 격리된 CPU에만 실행

log_info "트레이딩 스레드 CPU 고정 함수 제공..."

# 함수: taskset을 사용하여 프로세스를 특정 CPU에 고정
# 사용법: pin_to_cpu <pid> <cpu_list>
pin_to_cpu() {
    local pid=$1
    local cpus=$2

    if command -v taskset &> /dev/null; then
        if taskset -pc "$cpus" "$pid" 2>/dev/null; then
            log_info "PID $pid를 CPU $cpus에 고정했습니다."
        else
            log_error "PID $pid CPU 고정 실패"
            return 1
        fi
    else
        log_warn "taskset 명령을 찾을 수 없습니다."
        return 1
    fi
}

# 함수: 프로세스를 격리된 CPU에서 실행
# 사용법: run_isolated <command>
run_isolated() {
    if command -v taskset &> /dev/null; then
        taskset -c "$ISOLATED_CPUS" "$@"
    else
        log_warn "taskset이 없어서 CPU 격리 없이 실행합니다."
        "$@"
    fi
}

# ============================================================
# 8. 네트워크 튜닝
# ============================================================
log_info "네트워크 튜닝 중..."

# 소켓 버퍼 크기 증가 (네트워크 지연 감소)
NET_CONFIG="/etc/sysctl.conf"

if [ -f "$NET_CONFIG" ]; then
    # 수신 버퍼 (기본 128KB, 최대 128MB)
    sysctl -w net.core.rmem_default=16777216 2>/dev/null || log_warn "rmem_default 설정 실패"
    sysctl -w net.core.rmem_max=134217728 2>/dev/null || log_warn "rmem_max 설정 실패"

    # 송신 버퍼 (기본 128KB, 최대 128MB)
    sysctl -w net.core.wmem_default=16777216 2>/dev/null || log_warn "wmem_default 설정 실패"
    sysctl -w net.core.wmem_max=134217728 2>/dev/null || log_warn "wmem_max 설정 실패"

    # TCP 버퍼 튜닝
    # min default max
    sysctl -w net.ipv4.tcp_rmem="4096 16777216 134217728" 2>/dev/null || log_warn "tcp_rmem 설정 실패"
    sysctl -w net.ipv4.tcp_wmem="4096 16777216 134217728" 2>/dev/null || log_warn "tcp_wmem 설정 실패"

    # TCP_NODELAY 활성화 (Nagle 알고리즘 비활성화)
    # 이는 애플리케이션 수준에서 설정되어야 함

    # 네트백로그 증가
    sysctl -w net.ipv4.tcp_backlog=5000 2>/dev/null || log_warn "tcp_backlog 설정 실패"

    # TCP 연결 타임아웃 감소 (선택사항)
    sysctl -w net.ipv4.tcp_fin_timeout=30 2>/dev/null || log_warn "tcp_fin_timeout 설정 실패"

    log_info "네트워크 버퍼 설정:"
    sysctl net.core.rmem_max
    sysctl net.core.wmem_max
    sysctl net.ipv4.tcp_rmem
    sysctl net.ipv4.tcp_wmem
else
    log_warn "sysctl 설정 파일을 찾을 수 없습니다."
fi

# ============================================================
# 9. 디스크 I/O 스케줄러 설정
# ============================================================
log_info "디스크 I/O 스케줄러 설정..."

# 저지연 I/O를 위해 'none' 또는 'noop' 스케줄러 사용
# (최신 커널에서는 'none'이 권장됨)

for disk in /sys/block/*/queue/scheduler; do
    if [ -f "$disk" ]; then
        current=$(cat "$disk")
        if grep -q "none" <<< "$current"; then
            echo "none" > "$disk" 2>/dev/null || true
            log_info "$(basename $(dirname $(dirname $disk))): none으로 설정"
        elif grep -q "noop" <<< "$current"; then
            echo "noop" > "$disk" 2>/dev/null || true
            log_info "$(basename $(dirname $(dirname $disk))): noop으로 설정"
        fi
    fi
done

# ============================================================
# 10. 메모리 스왑 관련 설정
# ============================================================
log_info "메모리 스왑 설정..."

# swappiness 감소 (OOM 위험성 증가하지만 지연 감소)
# 0: 스왑 비활성화 (권장 안함 - OOM 위험)
# 10-20: 저지연 우선
sysctl -w vm.swappiness=10 2>/dev/null || log_warn "swappiness 설정 실패"

# vfs_cache_pressure 증가 (캐시 정리 빈도 증가)
sysctl -w vm.vfs_cache_pressure=200 2>/dev/null || log_warn "vfs_cache_pressure 설정 실패"

log_info "메모리 설정: swappiness=$(sysctl -n vm.swappiness)"

# ============================================================
# 11. 설정 내용 확인
# ============================================================
log_info "현재 설정 내용:"
echo ""
echo "=== CPU 정보 ==="
nproc
echo ""
echo "=== 격리 설정 ==="
echo "격리 CPU: $ISOLATED_CPUS"
echo "시스템 CPU: $SYSTEM_CPUS"
echo ""
echo "=== 부트 파라미터 (현재 커널) ==="
cat /proc/cmdline | grep -o "isolcpus=[^ ]*" || echo "isolcpus 파라미터 미설정"
echo ""
echo "=== 주요 네트워크 설정 ==="
sysctl net.core.rmem_max net.core.wmem_max net.ipv4.tcp_rmem net.ipv4.tcp_wmem 2>/dev/null || true
echo ""

# ============================================================
# 12. 최종 메시지
# ============================================================
log_info "CPU 격리 및 커널 튜닝 설정 완료"
echo ""
echo "=== 다음 단계 ==="
echo "1. 재부팅이 필요한 변경사항:"
echo "   - GRUB 부트 파라미터 수정 (isolcpus 등)"
echo "   - 수정한 후 'sudo reboot' 실행"
echo ""
echo "2. 트레이딩 애플리케이션 실행:"
echo "   - taskset -c $ISOLATED_CPUS ./your_trading_app"
echo "   또는"
echo "   - pin_to_cpu \$PID '$ISOLATED_CPUS'"
echo ""
echo "3. 성능 모니터링:"
echo "   - taskset -pc <pid>  (프로세스 CPU 확인)"
echo "   - numastat -n        (NUMA 메모리 통계)"
echo "   - ps aux | grep <app> (프로세스 확인)"
echo ""

exit 0
