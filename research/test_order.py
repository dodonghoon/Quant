"""
test_order.py — 업비트 KRW-BTC 시장가 매수 1,000원어치 테스트 주문
실행 전 확인: TRADING_MODE=live, 잔고 ≥ 1,000 KRW
"""

import sys
import uuid
import hashlib
import os
import json
import urllib.request
import urllib.parse
from pathlib import Path

# ── 1. .env.production 로드 ─────────────────────────────────────────────────
env_path = Path(__file__).parent.parent / "config" / ".env.production"
if env_path.exists():
    for line in env_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            k, v = line.split("=", 1)
            os.environ.setdefault(k.strip(), v.strip())

ACCESS_KEY = os.environ["UPBIT_ACCESS_KEY"]
SECRET_KEY = os.environ["UPBIT_SECRET_KEY"]
BASE_URL   = os.environ.get("UPBIT_BASE_URL", "https://api.upbit.com")

# ── 2. JWT 생성 (HS512 + query_hash) ────────────────────────────────────────
def make_jwt(params: dict) -> str:
    import hmac, base64

    # query string → SHA-512 hash
    query_string = urllib.parse.urlencode(params).encode("utf-8")
    query_hash = hashlib.sha512(query_string).hexdigest()

    header  = {"alg": "HS512", "typ": "JWT"}
    payload = {
        "access_key":        ACCESS_KEY,
        "nonce":             str(uuid.uuid4()),
        "query_hash":        query_hash,
        "query_hash_alg":    "SHA512",
    }

    def b64(d: dict) -> str:
        import json as _json
        return base64.urlsafe_b64encode(
            _json.dumps(d, separators=(",", ":")).encode()
        ).rstrip(b"=").decode()

    signing_input = f"{b64(header)}.{b64(payload)}".encode()
    signature = hmac.new(
        SECRET_KEY.encode("utf-8"), signing_input, hashlib.sha512
    ).digest()
    sig_b64 = base64.urlsafe_b64encode(signature).rstrip(b"=").decode()

    return f"{b64(header)}.{b64(payload)}.{sig_b64}"


# ── 3. 주문 실행 ────────────────────────────────────────────────────────────
def place_market_buy(market: str = "KRW-BTC", price_krw: int = 1000):
    """
    ord_type=price  → 금액 지정 시장가 매수 (price 파라미터 = 지불할 KRW)
    ord_type=market → 수량 지정 시장가 매도 (volume 파라미터 = 매도할 코인)
    """
    params = {
        "market":   market,
        "side":     "bid",      # bid=매수 / ask=매도
        "price":    str(price_krw),
        "ord_type": "price",    # 금액 지정 시장가
    }

    token = make_jwt(params)

    body = urllib.parse.urlencode(params).encode("utf-8")
    req  = urllib.request.Request(
        f"{BASE_URL}/v1/orders",
        data    = body,
        method  = "POST",
        headers = {
            "Authorization": f"Bearer {token}",
            "Content-Type":  "application/x-www-form-urlencoded",
        },
    )

    print(f"[ORDER] POST {BASE_URL}/v1/orders")
    print(f"        market={market}  side=bid  price={price_krw}원  ord_type=price")

    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            raw  = resp.read().decode("utf-8")
            data = json.loads(raw)

        print("\n[RESULT] 주문 성공 ✓")
        print(f"  주문 UUID  : {data.get('uuid', '-')}")
        print(f"  마켓       : {data.get('market', '-')}")
        print(f"  주문 타입   : {data.get('ord_type', '-')}")
        print(f"  주문 상태   : {data.get('state', '-')}")
        print(f"  주문 금액   : {data.get('price', '-')} KRW")
        print(f"  생성 시각   : {data.get('created_at', '-')}")
        return data

    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8")
        print(f"\n[ERROR] HTTP {e.code}: {e.reason}")
        try:
            err = json.loads(body)
            print(f"  에러 코드   : {err.get('error', {}).get('name', '-')}")
            print(f"  에러 메시지 : {err.get('error', {}).get('message', '-')}")
        except Exception:
            print(f"  Raw: {body}")
        sys.exit(1)

    except Exception as e:
        print(f"\n[ERROR] 예외 발생: {e}")
        sys.exit(1)


# ── 4. 잔고 확인 (선택) ─────────────────────────────────────────────────────
def check_krw_balance() -> float:
    payload = {
        "access_key": ACCESS_KEY,
        "nonce":      str(uuid.uuid4()),
    }
    import hmac, base64, json as _json

    def b64(d):
        return base64.urlsafe_b64encode(
            _json.dumps(d, separators=(",",":")).encode()
        ).rstrip(b"=").decode()

    header = {"alg": "HS512", "typ": "JWT"}
    signing_input = f"{b64(header)}.{b64(payload)}".encode()
    sig = hmac.new(SECRET_KEY.encode(), signing_input, hashlib.sha512).digest()
    token = f"{b64(header)}.{b64(payload)}.{base64.urlsafe_b64encode(sig).rstrip(b'=').decode()}"

    req = urllib.request.Request(
        f"{BASE_URL}/v1/accounts",
        headers={"Authorization": f"Bearer {token}"},
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            accounts = json.loads(resp.read().decode())
        for acc in accounts:
            if acc.get("currency") == "KRW":
                bal = float(acc.get("balance", 0))
                print(f"[BALANCE] 보유 KRW: {bal:,.0f}원  (locked: {float(acc.get('locked',0)):,.0f}원)")
                return bal
    except Exception as e:
        print(f"[WARN] 잔고 조회 실패: {e}")
    return 0.0


# ── 5. 메인 ─────────────────────────────────────────────────────────────────
if __name__ == "__main__":
    print("=" * 55)
    print("  업비트 시장가 매수 테스트 — KRW-BTC 1,000원")
    print("=" * 55)

    ORDER_KRW = 1_000  # 주문 금액 (원)
    MARKET    = "KRW-BTC"

    # 잔고 확인
    balance = check_krw_balance()
    if balance < ORDER_KRW:
        print(f"[ABORT] KRW 잔고 부족: {balance:,.0f}원 < {ORDER_KRW:,}원")
        sys.exit(1)

    # 주문 실행
    place_market_buy(market=MARKET, price_krw=ORDER_KRW)
    print("=" * 55)
