"""
Binance Fetcher 단위 테스트
============================
실제 API 호출 테스트 (네트워크 연결 필요) 및 캐시 로직 검증.

pytest research/tests/test_binance_fetcher.py
"""

from __future__ import annotations

import time
from pathlib import Path
from unittest.mock import patch, MagicMock

import pandas as pd
import pytest


# ---------------------------------------------------------------------------
# 심볼 매핑 테스트
# ---------------------------------------------------------------------------
class TestSymbolMapping:
    """업비트 → 바이낸스 심볼 매핑 테스트."""

    def test_krw_btc_mapping(self):
        """KRW-BTC → BTCUSDT 정상 변환."""
        from research.data.binance_fetcher import map_upbit_to_binance

        result = map_upbit_to_binance("KRW-BTC")
        assert result == "BTCUSDT"

    def test_krw_eth_mapping(self):
        """KRW-ETH → ETHUSDT 정상 변환."""
        from research.data.binance_fetcher import map_upbit_to_binance

        result = map_upbit_to_binance("KRW-ETH")
        assert result == "ETHUSDT"

    def test_non_krw_ticker_returns_none(self):
        """KRW 마켓이 아닌 티커 → None 반환."""
        from research.data.binance_fetcher import map_upbit_to_binance

        assert map_upbit_to_binance("BTC-ETH") is None
        assert map_upbit_to_binance("USDT-BTC") is None

    def test_invalid_ticker_returns_none(self):
        """존재하지 않는 형식 → None 반환."""
        from research.data.binance_fetcher import map_upbit_to_binance

        assert map_upbit_to_binance("INVALID") is None
        assert map_upbit_to_binance("") is None


# ---------------------------------------------------------------------------
# 실제 API 호출 테스트 (네트워크 필요)
# ---------------------------------------------------------------------------
@pytest.mark.network
class TestLiveAPI:
    """실제 바이낸스 API 호출 테스트. 네트워크 연결 필요."""

    def test_get_funding_rates_btc(self):
        """BTC 펀딩레이트 수집 및 DataFrame 검증."""
        from research.data.binance_fetcher import get_funding_rates

        df = get_funding_rates("BTCUSDT", limit=10)

        assert isinstance(df, pd.DataFrame)
        assert set(df.columns) == {"symbol", "fundingTime", "fundingRate"}
        assert len(df) > 0
        assert (df["symbol"] == "BTCUSDT").all()
        assert df["fundingRate"].dtype == float
        # fundingTime은 timezone-aware datetime이어야 함
        assert df["fundingTime"].dtype == "object" or hasattr(df["fundingTime"].iloc[0], "tzinfo")

    def test_get_funding_rates_eth(self):
        """ETH 펀딩레이트 수집 및 DataFrame 검증."""
        from research.data.binance_fetcher import get_funding_rates

        df = get_funding_rates("ETHUSDT", limit=10)

        assert isinstance(df, pd.DataFrame)
        assert len(df) > 0
        assert (df["symbol"] == "ETHUSDT").all()

    def test_get_open_interest_hist_btc(self):
        """BTC 미결제약정 수집 및 DataFrame 검증."""
        from research.data.binance_fetcher import get_open_interest_hist

        df = get_open_interest_hist("BTCUSDT", period="1h", limit=10)

        assert isinstance(df, pd.DataFrame)
        assert set(df.columns) == {"symbol", "timestamp", "sumOpenInterest", "sumOpenInterestValue"}
        assert len(df) > 0
        assert (df["symbol"] == "BTCUSDT").all()
        assert df["sumOpenInterest"].dtype == float
        assert df["sumOpenInterestValue"].dtype == float

    def test_get_open_interest_hist_eth(self):
        """ETH 미결제약정 수집 및 DataFrame 검증."""
        from research.data.binance_fetcher import get_open_interest_hist

        df = get_open_interest_hist("ETHUSDT", period="1h", limit=10)

        assert isinstance(df, pd.DataFrame)
        assert len(df) > 0
        assert (df["symbol"] == "ETHUSDT").all()

    def test_nonexistent_symbol_raises(self):
        """존재하지 않는 심볼 요청 시 HTTPError 발생."""
        import requests
        from research.data.binance_fetcher import get_funding_rates

        with pytest.raises(requests.HTTPError):
            get_funding_rates("FAKECOIN999USDT", limit=5)

    def test_funding_rate_kst_timezone(self):
        """fundingTime이 KST(UTC+9)로 변환되었는지 확인."""
        from research.data.binance_fetcher import get_funding_rates
        from datetime import timezone, timedelta

        df = get_funding_rates("BTCUSDT", limit=5)
        kst = timezone(timedelta(hours=9))

        # pandas datetime with timezone
        sample_time = pd.to_datetime(df["fundingTime"].iloc[0])
        assert sample_time.tzinfo is not None
        offset = sample_time.tzinfo.utcoffset(None)
        assert offset == timedelta(hours=9), f"KST offset 불일치: {offset}"


# ---------------------------------------------------------------------------
# 캐시 로직 테스트 (mock 사용)
# ---------------------------------------------------------------------------
class TestCacheLogic:
    """캐시 유효성 검사 및 fetch_all 캐시 분기 테스트."""

    def test_cache_valid_within_ttl(self, tmp_path: Path):
        """1시간 이내 수정된 CSV → 캐시 유효 반환."""
        from research.data import binance_fetcher as bf

        # 임시 CSV 파일 생성
        csv_path = tmp_path / "test.csv"
        csv_path.write_text("col1\n1\n")

        # _is_cache_valid는 경로 기반이므로 직접 호출
        assert bf._is_cache_valid(csv_path) is True

    def test_cache_invalid_old_file(self, tmp_path: Path):
        """1시간 초과 파일 → 캐시 무효."""
        import os
        from research.data import binance_fetcher as bf

        csv_path = tmp_path / "old.csv"
        csv_path.write_text("col1\n1\n")

        # mtime을 2시간 전으로 조작
        old_time = time.time() - 7300
        os.utime(csv_path, (old_time, old_time))

        assert bf._is_cache_valid(csv_path) is False

    def test_cache_missing_file(self, tmp_path: Path):
        """파일 없음 → 캐시 무효."""
        from research.data import binance_fetcher as bf

        assert bf._is_cache_valid(tmp_path / "nonexistent.csv") is False

    def test_fetch_all_uses_cache(self, tmp_path: Path):
        """캐시 유효 시 API 호출 없이 CSV 반환."""
        from research.data import binance_fetcher as bf

        # 임시 CSV 준비
        funding_csv = tmp_path / "funding_rates.csv"
        oi_csv = tmp_path / "open_interest.csv"

        funding_df = pd.DataFrame({
            "symbol": ["BTCUSDT"],
            "fundingTime": ["2024-01-01 09:00:00+09:00"],
            "fundingRate": [0.0001],
        })
        oi_df = pd.DataFrame({
            "symbol": ["BTCUSDT"],
            "timestamp": ["2024-01-01 09:00:00+09:00"],
            "sumOpenInterest": [1000.0],
            "sumOpenInterestValue": [50000000.0],
        })
        funding_df.to_csv(funding_csv, index=False)
        oi_df.to_csv(oi_csv, index=False)

        with (
            patch.object(bf, "_FUNDING_CSV", funding_csv),
            patch.object(bf, "_OI_CSV", oi_csv),
            patch.object(bf, "_is_cache_valid", return_value=True),
            patch("research.data.binance_fetcher.get_funding_rates") as mock_fr,
            patch("research.data.binance_fetcher.get_open_interest_hist") as mock_oi,
        ):
            result_fr, result_oi = bf.fetch_all(["KRW-BTC"], force_refresh=False)

        # 캐시 사용 → API 호출 없어야 함
        mock_fr.assert_not_called()
        mock_oi.assert_not_called()

        assert isinstance(result_fr, pd.DataFrame)
        assert isinstance(result_oi, pd.DataFrame)

    def test_fetch_all_force_refresh_ignores_cache(self, tmp_path: Path):
        """force_refresh=True이면 캐시 무시하고 API 호출."""
        from research.data import binance_fetcher as bf

        mock_funding = pd.DataFrame({
            "symbol": ["BTCUSDT"],
            "fundingTime": [pd.Timestamp("2024-01-01", tz="Asia/Seoul")],
            "fundingRate": [0.0001],
        })
        mock_oi = pd.DataFrame({
            "symbol": ["BTCUSDT"],
            "timestamp": [pd.Timestamp("2024-01-01", tz="Asia/Seoul")],
            "sumOpenInterest": [1000.0],
            "sumOpenInterestValue": [50000000.0],
        })

        with (
            patch.object(bf, "_DATA_DIR", tmp_path),
            patch.object(bf, "_FUNDING_CSV", tmp_path / "funding_rates.csv"),
            patch.object(bf, "_OI_CSV", tmp_path / "open_interest.csv"),
            patch("research.data.binance_fetcher.get_funding_rates", return_value=mock_funding) as mock_fr,
            patch("research.data.binance_fetcher.get_open_interest_hist", return_value=mock_oi) as mock_oi_fn,
        ):
            result_fr, result_oi = bf.fetch_all(["KRW-BTC"], force_refresh=True)

        # force_refresh → API 호출됨
        mock_fr.assert_called_once_with("BTCUSDT")
        mock_oi_fn.assert_called_once_with("BTCUSDT")

    def test_fetch_all_skips_unmappable_tickers(self, tmp_path: Path):
        """바이낸스 매핑 불가 티커는 자동 스킵 (예외 없음)."""
        from research.data import binance_fetcher as bf

        with (
            patch.object(bf, "_DATA_DIR", tmp_path),
            patch.object(bf, "_FUNDING_CSV", tmp_path / "funding_rates.csv"),
            patch.object(bf, "_OI_CSV", tmp_path / "open_interest.csv"),
            patch("research.data.binance_fetcher.get_funding_rates") as mock_fr,
        ):
            # 매핑 불가 티커만 전달
            result_fr, result_oi = bf.fetch_all(["BTC-WEIRDCOIN"], force_refresh=True)

        mock_fr.assert_not_called()
        assert result_fr.empty
        assert result_oi.empty

    def test_fetch_all_continues_on_api_error(self, tmp_path: Path):
        """개별 심볼 API 실패 시 경고만 출력하고 계속 진행."""
        import requests
        from research.data import binance_fetcher as bf

        with (
            patch.object(bf, "_DATA_DIR", tmp_path),
            patch.object(bf, "_FUNDING_CSV", tmp_path / "funding_rates.csv"),
            patch.object(bf, "_OI_CSV", tmp_path / "open_interest.csv"),
            patch(
                "research.data.binance_fetcher.get_funding_rates",
                side_effect=requests.ConnectionError("연결 오류"),
            ),
            patch(
                "research.data.binance_fetcher.get_open_interest_hist",
                side_effect=requests.ConnectionError("연결 오류"),
            ),
        ):
            # 예외 없이 완료되어야 함
            result_fr, result_oi = bf.fetch_all(["KRW-BTC", "KRW-ETH"], force_refresh=True)

        assert isinstance(result_fr, pd.DataFrame)
        assert result_fr.empty
