"""
Data Lake — Zarr/Parquet 기반 틱 데이터 저장소

기술문서 §5.1 "Research Archives: Zarr 또는 Apache Parquet" 구현.
대용량 틱 데이터의 효율적 압축 및 시계열 슬라이싱을 지원합니다.

사용 예시:
    store = TickStore("./data")
    store.ingest_csv("btcusdt_trades.csv", symbol="BTCUSDT", exchange="binance")
    df = store.query("BTCUSDT", start="2024-01-01", end="2024-02-01")
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Literal

import numpy as np
import pandas as pd
import zarr
from loguru import logger


class TickStore:
    """Zarr + Parquet 이중 저장소.

    - Zarr: 다차원 배열 기반, 시계열 슬라이싱에 최적 (연구용 고속 랜덤 액세스)
    - Parquet: 컬럼형 저장, pandas/polars와의 호환성 (공유 및 외부 도구 연동)

    디렉토리 구조:
        {root}/
        ├── zarr/
        │   └── {exchange}/{symbol}/        ← Zarr DirectoryStore
        │       ├── timestamp (int64, ns)
        │       ├── price     (float64)
        │       ├── quantity  (float64)
        │       └── side      (int8: 0=bid, 1=ask)
        └── parquet/
            └── {exchange}/{symbol}/
                └── {YYYY-MM-DD}.parquet    ← 일별 파티션
    """

    # 틱 데이터 스키마 (Rust types.rs의 Trade 구조체와 일치)
    COLUMNS = ["timestamp_ns", "price", "quantity", "side", "exchange_ts_ns"]

    def __init__(self, root: str | Path) -> None:
        self.root = Path(root)
        self.zarr_root = self.root / "zarr"
        self.parquet_root = self.root / "parquet"
        self.zarr_root.mkdir(parents=True, exist_ok=True)
        self.parquet_root.mkdir(parents=True, exist_ok=True)
        logger.info(f"TickStore initialized at {self.root}")

    # ─────────────────────────────────────────
    # Ingestion
    # ─────────────────────────────────────────

    def ingest_csv(
        self,
        csv_path: str | Path,
        symbol: str,
        exchange: str = "binance",
        timestamp_col: str = "timestamp",
        timestamp_unit: Literal["ns", "us", "ms", "s"] = "ms",
    ) -> int:
        """CSV 파일을 Zarr + Parquet 저장소에 적재.

        Args:
            csv_path: 원본 CSV 경로
            symbol: 심볼명 (e.g., "BTCUSDT")
            exchange: 거래소 (e.g., "binance", "upbit")
            timestamp_col: 타임스탬프 컬럼명
            timestamp_unit: 타임스탬프 단위

        Returns:
            적재된 레코드 수
        """
        df = pd.read_csv(csv_path)
        logger.info(f"Read {len(df)} rows from {csv_path}")

        # 타임스탬프 정규화 → 나노초
        ts = pd.to_numeric(df[timestamp_col])
        multipliers = {"s": 10**9, "ms": 10**6, "us": 10**3, "ns": 1}
        df["timestamp_ns"] = (ts * multipliers[timestamp_unit]).astype(np.int64)

        return self._store(df, symbol, exchange)

    def ingest_dataframe(
        self,
        df: pd.DataFrame,
        symbol: str,
        exchange: str = "binance",
    ) -> int:
        """pandas DataFrame을 직접 적재.

        필수 컬럼: timestamp_ns, price, quantity, side
        side: 0 = Bid, 1 = Ask (Rust types.rs Side enum과 일치)
        """
        required = {"timestamp_ns", "price", "quantity", "side"}
        missing = required - set(df.columns)
        if missing:
            raise ValueError(f"Missing columns: {missing}")

        return self._store(df, symbol, exchange)

    def _store(self, df: pd.DataFrame, symbol: str, exchange: str) -> int:
        """Zarr + Parquet에 동시 기록."""
        n = len(df)
        if n == 0:
            return 0

        self._store_zarr(df, symbol, exchange)
        self._store_parquet(df, symbol, exchange)

        logger.info(f"Stored {n} ticks for {exchange}/{symbol}")
        return n

    def _store_zarr(self, df: pd.DataFrame, symbol: str, exchange: str) -> None:
        """Zarr DirectoryStore에 append."""
        store_path = self.zarr_root / exchange / symbol
        store = zarr.DirectoryStore(str(store_path))
        root = zarr.group(store=store, overwrite=False)

        for col in ["timestamp_ns", "price", "quantity"]:
            if col not in df.columns:
                continue
            arr = df[col].values
            dtype = np.int64 if col == "timestamp_ns" else np.float64

            if col in root:
                existing = root[col]
                existing.append(arr.astype(dtype))
            else:
                root.create_dataset(
                    col,
                    data=arr.astype(dtype),
                    chunks=(min(100_000, len(arr)),),
                    dtype=dtype,
                    compressor=zarr.Blosc(cname="zstd", clevel=3),
                )

        # side: int8 (0=bid, 1=ask)
        if "side" in df.columns:
            side_arr = df["side"].values.astype(np.int8)
            if "side" in root:
                root["side"].append(side_arr)
            else:
                root.create_dataset(
                    "side",
                    data=side_arr,
                    chunks=(min(100_000, len(side_arr)),),
                    dtype=np.int8,
                    compressor=zarr.Blosc(cname="zstd", clevel=3),
                )

    def _store_parquet(self, df: pd.DataFrame, symbol: str, exchange: str) -> None:
        """일별 파티션 Parquet으로 저장."""
        out_dir = self.parquet_root / exchange / symbol
        out_dir.mkdir(parents=True, exist_ok=True)

        # 나노초 → 날짜 파티션
        df = df.copy()
        df["_date"] = pd.to_datetime(df["timestamp_ns"], unit="ns").dt.date

        for date, group in df.groupby("_date"):
            path = out_dir / f"{date}.parquet"
            cols = [c for c in self.COLUMNS if c in group.columns]
            partition = group[cols]

            if path.exists():
                existing = pd.read_parquet(path)
                partition = pd.concat([existing, partition], ignore_index=True)
                partition = partition.sort_values("timestamp_ns").drop_duplicates(
                    subset=["timestamp_ns"], keep="last"
                )

            partition.to_parquet(path, engine="pyarrow", compression="zstd", index=False)

    # ─────────────────────────────────────────
    # Query
    # ─────────────────────────────────────────

    def query(
        self,
        symbol: str,
        start: str | None = None,
        end: str | None = None,
        exchange: str = "binance",
        backend: Literal["zarr", "parquet"] = "parquet",
    ) -> pd.DataFrame:
        """시간 범위 조회.

        Args:
            symbol: 심볼명
            start: 시작 시각 (ISO 8601, 예: "2024-01-01")
            end: 종료 시각
            exchange: 거래소
            backend: 저장소 선택

        Returns:
            틱 데이터 DataFrame
        """
        if backend == "parquet":
            return self._query_parquet(symbol, exchange, start, end)
        return self._query_zarr(symbol, exchange, start, end)

    def _query_parquet(
        self, symbol: str, exchange: str, start: str | None, end: str | None
    ) -> pd.DataFrame:
        pq_dir = self.parquet_root / exchange / symbol
        if not pq_dir.exists():
            return pd.DataFrame(columns=self.COLUMNS)

        files = sorted(pq_dir.glob("*.parquet"))
        if start:
            start_date = pd.Timestamp(start).date()
            files = [f for f in files if f.stem >= str(start_date)]
        if end:
            end_date = pd.Timestamp(end).date()
            files = [f for f in files if f.stem <= str(end_date)]

        if not files:
            return pd.DataFrame(columns=self.COLUMNS)

        dfs = [pd.read_parquet(f) for f in files]
        df = pd.concat(dfs, ignore_index=True)

        # 나노초 범위 필터
        if start:
            start_ns = int(pd.Timestamp(start).timestamp() * 1e9)
            df = df[df["timestamp_ns"] >= start_ns]
        if end:
            end_ns = int(pd.Timestamp(end).timestamp() * 1e9)
            df = df[df["timestamp_ns"] <= end_ns]

        return df.sort_values("timestamp_ns").reset_index(drop=True)

    def _query_zarr(
        self, symbol: str, exchange: str, start: str | None, end: str | None
    ) -> pd.DataFrame:
        store_path = self.zarr_root / exchange / symbol
        if not store_path.exists():
            return pd.DataFrame(columns=self.COLUMNS)

        store = zarr.DirectoryStore(str(store_path))
        root = zarr.open_group(store=store, mode="r")

        ts = root["timestamp_ns"][:]
        mask = np.ones(len(ts), dtype=bool)

        if start:
            start_ns = int(pd.Timestamp(start).timestamp() * 1e9)
            mask &= ts >= start_ns
        if end:
            end_ns = int(pd.Timestamp(end).timestamp() * 1e9)
            mask &= ts <= end_ns

        data = {"timestamp_ns": ts[mask]}
        for col in ["price", "quantity", "side"]:
            if col in root:
                data[col] = root[col][:][mask]

        return pd.DataFrame(data).sort_values("timestamp_ns").reset_index(drop=True)

    # ─────────────────────────────────────────
    # Utilities
    # ─────────────────────────────────────────

    def to_ohlcv(
        self,
        symbol: str,
        freq: str = "1min",
        exchange: str = "binance",
        start: str | None = None,
        end: str | None = None,
    ) -> pd.DataFrame:
        """틱 데이터 → OHLCV 캔들로 리샘플링.

        vectorbt 백테스팅 입력에 사용됩니다.
        """
        df = self.query(symbol, start=start, end=end, exchange=exchange)
        if df.empty:
            return df

        df["datetime"] = pd.to_datetime(df["timestamp_ns"], unit="ns")
        df = df.set_index("datetime")

        ohlcv = df["price"].resample(freq).agg(
            open="first", high="max", low="min", close="last"
        )
        ohlcv["volume"] = df["quantity"].resample(freq).sum()
        return ohlcv.dropna()

    def info(self, exchange: str | None = None) -> dict:
        """저장소 메타정보 반환."""
        result = {}
        search_root = self.parquet_root / exchange if exchange else self.parquet_root

        for exch_dir in sorted(search_root.iterdir()):
            if not exch_dir.is_dir():
                continue
            exch_name = exch_dir.name
            result[exch_name] = {}
            for sym_dir in sorted(exch_dir.iterdir()):
                if not sym_dir.is_dir():
                    continue
                files = list(sym_dir.glob("*.parquet"))
                total_size = sum(f.stat().st_size for f in files)
                result[exch_name][sym_dir.name] = {
                    "files": len(files),
                    "size_mb": round(total_size / (1024 * 1024), 2),
                    "date_range": (
                        f"{sorted(f.stem for f in files)[0]} ~ "
                        f"{sorted(f.stem for f in files)[-1]}"
                    )
                    if files
                    else "empty",
                }
        return result
