"""
ML Training — PyTorch LSTM/Transformer + ONNX 내보내기

기술문서 §4.2.3 구현:
    "Transformer/Attention: 시장 미시구조 처리.
     Python에서 학습된 모델을 ONNX 포맷으로 내보낸 후,
     Rust의 ort 크레이트(ONNX Runtime)로 로드하여 실행."

워크플로우:
    1. Python에서 PyTorch 모델 학습
    2. ONNX 포맷으로 내보내기
    3. Rust execution-engine에서 ort 크레이트로 추론

사용 예시:
    predictor = PricePredictor(seq_len=60, hidden_dim=64)
    predictor.train(train_data, epochs=50)
    predictor.export_onnx("model.onnx")
"""

from __future__ import annotations

import math
from pathlib import Path
from typing import Literal

import numpy as np
import pandas as pd
import torch
import torch.nn as nn
from loguru import logger
from torch.utils.data import DataLoader, Dataset


# ─────────────────────────────────────────
# Dataset
# ─────────────────────────────────────────

class TimeSeriesDataset(Dataset):
    """시계열 슬라이딩 윈도우 데이터셋.

    Rust features.rs의 RollingWindow와 동일한 윈도우 개념:
    [t-seq_len : t] → predict [t+1 : t+horizon]
    """

    def __init__(
        self,
        data: np.ndarray,
        seq_len: int = 60,
        horizon: int = 1,
        feature_cols: int = 1,
    ) -> None:
        self.data = torch.FloatTensor(data)
        self.seq_len = seq_len
        self.horizon = horizon
        self.n_features = feature_cols

    def __len__(self) -> int:
        return len(self.data) - self.seq_len - self.horizon + 1

    def __getitem__(self, idx: int):
        x = self.data[idx : idx + self.seq_len]
        y = self.data[idx + self.seq_len : idx + self.seq_len + self.horizon]

        # 단일 피처 → (seq_len, 1) 형태로 보장
        if x.dim() == 1:
            x = x.unsqueeze(-1)
        if y.dim() == 1:
            y = y.unsqueeze(-1)

        return x, y[:, 0] if y.shape[-1] >= 1 else y  # 가격 컬럼만 타겟


# ─────────────────────────────────────────
# LSTM Model
# ─────────────────────────────────────────

class LSTMModel(nn.Module):
    """LSTM 가격 예측 모델.

    구조: Input → LSTM layers → Linear → Output
    """

    def __init__(
        self,
        input_dim: int = 1,
        hidden_dim: int = 64,
        num_layers: int = 2,
        output_dim: int = 1,
        dropout: float = 0.1,
    ) -> None:
        super().__init__()
        self.hidden_dim = hidden_dim
        self.num_layers = num_layers

        self.lstm = nn.LSTM(
            input_size=input_dim,
            hidden_size=hidden_dim,
            num_layers=num_layers,
            batch_first=True,
            dropout=dropout if num_layers > 1 else 0.0,
        )
        self.fc = nn.Linear(hidden_dim, output_dim)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        # x: (batch, seq_len, input_dim)
        lstm_out, _ = self.lstm(x)
        # 마지막 타임스텝만 사용
        out = self.fc(lstm_out[:, -1, :])
        return out


# ─────────────────────────────────────────
# Transformer Model
# ─────────────────────────────────────────

class PositionalEncoding(nn.Module):
    """Sinusoidal Positional Encoding (Attention Is All You Need)."""

    def __init__(self, d_model: int, max_len: int = 500, dropout: float = 0.1):
        super().__init__()
        self.dropout = nn.Dropout(p=dropout)

        pe = torch.zeros(max_len, d_model)
        position = torch.arange(0, max_len, dtype=torch.float).unsqueeze(1)
        div_term = torch.exp(
            torch.arange(0, d_model, 2).float() * (-math.log(10000.0) / d_model)
        )
        pe[:, 0::2] = torch.sin(position * div_term)
        pe[:, 1::2] = torch.cos(position * div_term)
        pe = pe.unsqueeze(0)  # (1, max_len, d_model)
        self.register_buffer("pe", pe)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        x = x + self.pe[:, : x.size(1)]
        return self.dropout(x)


class TransformerModel(nn.Module):
    """Transformer 가격 예측 모델.

    기술문서: "Transformer/Attention: 시장 미시구조(Market Microstructure)
    나 뉴스 텍스트 처리에 활용"

    구조: Input → Linear projection → Positional Encoding
          → Transformer Encoder → Linear → Output
    """

    def __init__(
        self,
        input_dim: int = 1,
        d_model: int = 64,
        n_heads: int = 4,
        n_layers: int = 2,
        dim_feedforward: int = 128,
        output_dim: int = 1,
        dropout: float = 0.1,
        max_len: int = 500,
    ) -> None:
        super().__init__()

        self.input_proj = nn.Linear(input_dim, d_model)
        self.pos_encoder = PositionalEncoding(d_model, max_len, dropout)

        encoder_layer = nn.TransformerEncoderLayer(
            d_model=d_model,
            nhead=n_heads,
            dim_feedforward=dim_feedforward,
            dropout=dropout,
            batch_first=True,
        )
        self.transformer_encoder = nn.TransformerEncoder(
            encoder_layer, num_layers=n_layers
        )

        self.fc = nn.Linear(d_model, output_dim)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        # x: (batch, seq_len, input_dim)
        x = self.input_proj(x)       # → (batch, seq_len, d_model)
        x = self.pos_encoder(x)
        x = self.transformer_encoder(x)
        out = self.fc(x[:, -1, :])   # 마지막 타임스텝
        return out


# ─────────────────────────────────────────
# Trainer
# ─────────────────────────────────────────

class PricePredictor:
    """가격 예측 모델 학습 및 ONNX 내보내기.

    Rust ort 크레이트와의 연동을 위해 ONNX export를 기본 지원합니다.
    """

    def __init__(
        self,
        model_type: Literal["lstm", "transformer"] = "lstm",
        seq_len: int = 60,
        hidden_dim: int = 64,
        num_layers: int = 2,
        n_heads: int = 4,
        lr: float = 1e-3,
        device: str | None = None,
    ) -> None:
        self.model_type = model_type
        self.seq_len = seq_len
        self.hidden_dim = hidden_dim
        self.device = device or ("cuda" if torch.cuda.is_available() else "cpu")

        if model_type == "lstm":
            self.model = LSTMModel(
                input_dim=1,
                hidden_dim=hidden_dim,
                num_layers=num_layers,
                output_dim=1,
            ).to(self.device)
        else:
            self.model = TransformerModel(
                input_dim=1,
                d_model=hidden_dim,
                n_heads=n_heads,
                n_layers=num_layers,
                output_dim=1,
            ).to(self.device)

        self.optimizer = torch.optim.Adam(self.model.parameters(), lr=lr)
        self.criterion = nn.MSELoss()

        # 정규화 파라미터 (ONNX 추론 시에도 동일 적용 필요)
        self._mean: float = 0.0
        self._std: float = 1.0

        logger.info(
            f"PricePredictor({model_type}): seq_len={seq_len}, "
            f"hidden={hidden_dim}, device={self.device}"
        )

    def _normalize(self, data: np.ndarray) -> np.ndarray:
        """Z-score 정규화 (Rust features.rs z_score와 동일)."""
        self._mean = float(np.mean(data))
        self._std = float(np.std(data))
        if self._std < 1e-15:
            self._std = 1.0
        return (data - self._mean) / self._std

    def _denormalize(self, data: np.ndarray) -> np.ndarray:
        return data * self._std + self._mean

    def train(
        self,
        prices: pd.Series | np.ndarray,
        epochs: int = 50,
        batch_size: int = 64,
        val_split: float = 0.2,
    ) -> dict:
        """모델 학습.

        Args:
            prices: 가격 시계열
            epochs: 학습 에폭 수
            batch_size: 배치 크기
            val_split: 검증 데이터 비율

        Returns:
            학습 이력 (train_loss, val_loss per epoch)
        """
        data = np.asarray(prices, dtype=np.float64)
        data = self._normalize(data)

        # Train/Val 분할 (시계열이므로 순서 유지)
        split_idx = int(len(data) * (1 - val_split))
        train_ds = TimeSeriesDataset(data[:split_idx], seq_len=self.seq_len)
        val_ds = TimeSeriesDataset(data[split_idx:], seq_len=self.seq_len)

        train_loader = DataLoader(train_ds, batch_size=batch_size, shuffle=True)
        val_loader = DataLoader(val_ds, batch_size=batch_size, shuffle=False)

        logger.info(
            f"Training: {len(train_ds)} train, {len(val_ds)} val samples"
        )

        history = {"train_loss": [], "val_loss": []}

        for epoch in range(epochs):
            # ── Train ──
            self.model.train()
            train_losses = []
            for x_batch, y_batch in train_loader:
                x_batch = x_batch.to(self.device)
                y_batch = y_batch.to(self.device)

                pred = self.model(x_batch)
                loss = self.criterion(pred.squeeze(), y_batch.squeeze())

                self.optimizer.zero_grad()
                loss.backward()
                torch.nn.utils.clip_grad_norm_(self.model.parameters(), max_norm=1.0)
                self.optimizer.step()
                train_losses.append(loss.item())

            # ── Validation ──
            self.model.eval()
            val_losses = []
            with torch.no_grad():
                for x_batch, y_batch in val_loader:
                    x_batch = x_batch.to(self.device)
                    y_batch = y_batch.to(self.device)
                    pred = self.model(x_batch)
                    loss = self.criterion(pred.squeeze(), y_batch.squeeze())
                    val_losses.append(loss.item())

            avg_train = np.mean(train_losses)
            avg_val = np.mean(val_losses) if val_losses else float("nan")
            history["train_loss"].append(avg_train)
            history["val_loss"].append(avg_val)

            if (epoch + 1) % 10 == 0 or epoch == 0:
                logger.info(
                    f"Epoch {epoch+1}/{epochs} — "
                    f"train_loss: {avg_train:.6f}, val_loss: {avg_val:.6f}"
                )

        return history

    def predict(self, prices: pd.Series | np.ndarray) -> np.ndarray:
        """주어진 시퀀스로 다음 값 예측."""
        data = np.asarray(prices, dtype=np.float64)
        data = (data - self._mean) / self._std

        self.model.eval()
        with torch.no_grad():
            x = torch.FloatTensor(data[-self.seq_len :]).unsqueeze(0).unsqueeze(-1)
            x = x.to(self.device)
            pred = self.model(x).cpu().numpy()

        return self._denormalize(pred.flatten())

    def export_onnx(
        self,
        path: str | Path,
        opset_version: int = 14,
    ) -> Path:
        """ONNX 포맷으로 모델 내보내기.

        Rust의 ort 크레이트로 로드하여 실시간 추론에 사용됩니다.
        기술문서 §4.2.3: "ONNX 포맷으로 내보낸 후 Rust의 ort 크레이트로 로드"

        내보내기 파일과 함께 정규화 파라미터도 저장합니다.
        """
        path = Path(path)
        self.model.eval()
        self.model.to("cpu")

        # 더미 입력: (batch=1, seq_len, features=1)
        dummy_input = torch.randn(1, self.seq_len, 1)

        torch.onnx.export(
            self.model,
            dummy_input,
            str(path),
            export_params=True,
            opset_version=opset_version,
            do_constant_folding=True,
            input_names=["input"],
            output_names=["output"],
            dynamic_axes={
                "input": {0: "batch_size"},
                "output": {0: "batch_size"},
            },
        )

        # 정규화 파라미터 저장 (Rust 추론 시 동일 적용 필요)
        meta_path = path.with_suffix(".json")
        import json
        meta = {
            "model_type": self.model_type,
            "seq_len": self.seq_len,
            "hidden_dim": self.hidden_dim,
            "normalize_mean": self._mean,
            "normalize_std": self._std,
        }
        meta_path.write_text(json.dumps(meta, indent=2))

        self.model.to(self.device)

        logger.info(f"ONNX exported: {path} + {meta_path}")
        logger.info(f"  Normalize: mean={self._mean:.6f}, std={self._std:.6f}")
        return path

    def validate_onnx(self, path: str | Path) -> bool:
        """ONNX 모델 검증 (로드 및 추론 테스트).

        Rust ort 크레이트로 로드하기 전에 Python에서 먼저 검증합니다.
        """
        try:
            import onnx
            import onnxruntime as ort

            # 구조 검증
            model = onnx.load(str(path))
            onnx.checker.check_model(model)

            # 추론 테스트
            session = ort.InferenceSession(str(path))
            dummy = np.random.randn(1, self.seq_len, 1).astype(np.float32)
            result = session.run(None, {"input": dummy})

            logger.info(
                f"ONNX validation passed: output shape = {result[0].shape}"
            )
            return True

        except Exception as e:
            logger.error(f"ONNX validation failed: {e}")
            return False
