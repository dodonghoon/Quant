"""
연구 계층 단위 테스트 — 외부 서비스 없이 순수 수학 검증

pytest research/tests/test_models.py
"""

import numpy as np
import pandas as pd
import pytest


class TestCointegration:
    """공적분 검정 테스트 (Rust ou_model.rs 파라미터와 일치 확인)."""

    def test_cointegrated_pair(self):
        """합성 공적분 데이터 → is_cointegrated = True."""
        from research.models.cointegration import CointegrationTester

        np.random.seed(42)
        n = 1000

        # 공통 랜덤 워크 + 평균회귀 스프레드
        random_walk = np.cumsum(np.random.randn(n) * 0.5)
        price_b = 100 + random_walk
        noise = np.zeros(n)
        for i in range(1, n):
            noise[i] = 0.8 * noise[i - 1] + np.random.randn() * 0.3  # AR(1), |ρ|<1
        price_a = 2.0 * price_b + 10.0 + noise  # β=2.0, 공적분

        tester = CointegrationTester(significance=0.05)
        result = tester.engle_granger(price_a, price_b)

        assert result.is_cointegrated, f"Should be cointegrated, p={result.p_value}"
        assert result.p_value < 0.05
        assert abs(result.hedge_ratio - 2.0) < 0.5  # β ≈ 2.0
        assert result.half_life < 100  # 유한 half-life
        assert result.kappa > 0.01  # Rust min_kappa threshold

    def test_non_cointegrated_pair(self):
        """독립 랜덤 워크 → is_cointegrated = False."""
        from research.models.cointegration import CointegrationTester

        np.random.seed(123)
        n = 500
        price_a = 100 + np.cumsum(np.random.randn(n))
        price_b = 200 + np.cumsum(np.random.randn(n))

        tester = CointegrationTester()
        result = tester.engle_granger(price_a, price_b)

        assert not result.is_cointegrated

    def test_ou_params_consistency(self):
        """OU 파라미터가 Rust ou_model.rs 검증 기준과 일치."""
        from research.models.cointegration import CointegrationTester

        np.random.seed(7)
        n = 2000
        # 명확한 OU 프로세스: κ=0.1, μ=0, σ=1
        spread = np.zeros(n)
        kappa_true = 0.1
        for i in range(1, n):
            spread[i] = spread[i - 1] - kappa_true * spread[i - 1] + np.random.randn()

        tester = CointegrationTester()
        # 더미 pair 생성
        price_b = np.ones(n) * 100
        price_a = price_b + spread

        result = tester.engle_granger(price_a, price_b)

        # half_life = ln(2)/κ ≈ 6.93
        assert result.kappa > 0, "kappa should be positive"
        assert result.half_life < 86400, "Rust max_half_life = 86400"


class TestGarch:
    """GARCH 모델 테스트."""

    def test_garch_fit(self):
        """합성 GARCH 데이터 적합."""
        from research.models.garch import GarchModeler

        np.random.seed(42)
        n = 1000
        returns = np.random.randn(n) * 0.01  # 일별 수익률 1% 변동성

        modeler = GarchModeler()
        result = modeler.fit(pd.Series(returns))

        assert result.params.alpha >= 0
        assert result.params.beta >= 0
        assert result.params.persistence < 1.0  # 정상성 조건
        assert result.conditional_volatility is not None

    def test_garch_forecast(self):
        """GARCH 변동성 예측."""
        from research.models.garch import GarchModeler

        np.random.seed(42)
        returns = pd.Series(np.random.randn(500) * 0.02)

        modeler = GarchModeler()
        modeler.fit(returns)
        forecast = modeler.forecast(horizon=5)

        assert forecast.horizon == 5
        assert len(forecast.volatility_forecast) == 5
        assert all(v > 0 for v in forecast.volatility_forecast)

    def test_garch_half_life(self):
        """변동성 half-life 계산 (Rust ou_model.rs ln(2)/κ와 동일 원리)."""
        from research.models.garch import GarchParams

        params = GarchParams(omega=0.00001, alpha=0.05, beta=0.90)
        assert params.persistence == pytest.approx(0.95, abs=1e-10)
        assert params.half_life > 0
        assert params.half_life == pytest.approx(
            np.log(2) / np.log(1 / 0.95), abs=0.01
        )


class TestDataLake:
    """Data Lake 저장소 테스트."""

    def test_roundtrip(self, tmp_path):
        """DataFrame 저장 후 조회 → 동일 데이터."""
        from research.data_lake.store import TickStore

        store = TickStore(tmp_path / "data")
        df = pd.DataFrame({
            "timestamp_ns": [
                1700000000_000_000_000,
                1700000001_000_000_000,
                1700000002_000_000_000,
            ],
            "price": [50000.0, 50001.0, 50002.0],
            "quantity": [0.1, 0.2, 0.3],
            "side": [0, 1, 0],
        })

        n = store.ingest_dataframe(df, symbol="BTCUSDT", exchange="binance")
        assert n == 3

        result = store.query("BTCUSDT", exchange="binance", backend="parquet")
        assert len(result) == 3
        assert result["price"].tolist() == [50000.0, 50001.0, 50002.0]

    def test_ohlcv_resample(self, tmp_path):
        """틱 → OHLCV 변환."""
        from research.data_lake.store import TickStore

        store = TickStore(tmp_path / "data")

        # 1분간 틱 데이터 생성
        base_ts = 1700000000_000_000_000
        n_ticks = 100
        df = pd.DataFrame({
            "timestamp_ns": [base_ts + i * 500_000_000 for i in range(n_ticks)],
            "price": np.random.uniform(50000, 50100, n_ticks),
            "quantity": np.random.uniform(0.01, 1.0, n_ticks),
            "side": np.random.choice([0, 1], n_ticks),
        })

        store.ingest_dataframe(df, "BTCUSDT")
        ohlcv = store.to_ohlcv("BTCUSDT", freq="1min")
        assert "open" in ohlcv.columns
        assert "volume" in ohlcv.columns


class TestMLTraining:
    """ML Training 모듈 테스트 (경량 스모크 테스트)."""

    def test_lstm_forward_pass(self):
        """LSTM 모델 forward pass."""
        from research.models.ml_training import LSTMModel

        model = LSTMModel(input_dim=1, hidden_dim=16, num_layers=1)
        x = torch.randn(4, 30, 1)  # (batch=4, seq=30, features=1)
        out = model(x)
        assert out.shape == (4, 1)

    def test_transformer_forward_pass(self):
        """Transformer 모델 forward pass."""
        from research.models.ml_training import TransformerModel

        model = TransformerModel(input_dim=1, d_model=16, n_heads=2, n_layers=1)
        x = torch.randn(4, 30, 1)
        out = model(x)
        assert out.shape == (4, 1)

    def test_dataset_windowing(self):
        """TimeSeriesDataset 윈도우 슬라이싱."""
        from research.models.ml_training import TimeSeriesDataset

        data = np.arange(100, dtype=np.float32)
        ds = TimeSeriesDataset(data, seq_len=10, horizon=1)

        assert len(ds) == 90  # 100 - 10 - 1 + 1
        x, y = ds[0]
        assert x.shape == (10, 1)

    def test_export_onnx(self, tmp_path):
        """ONNX 내보내기 테스트."""
        from research.models.ml_training import PricePredictor

        predictor = PricePredictor(model_type="lstm", seq_len=20, hidden_dim=8)

        # 더미 학습 (1 epoch)
        prices = np.cumsum(np.random.randn(200)) + 100
        predictor.train(prices, epochs=1, batch_size=16)

        # ONNX export
        onnx_path = tmp_path / "test_model.onnx"
        predictor.export_onnx(onnx_path)
        assert onnx_path.exists()
        assert onnx_path.with_suffix(".json").exists()
