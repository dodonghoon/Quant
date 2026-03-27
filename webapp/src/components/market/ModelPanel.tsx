'use client';

interface ModelPanelProps {
  symbol: string;
}

export default function ModelPanel({ symbol }: ModelPanelProps) {
  return (
    <div className="rounded-lg border border-gray-800 bg-bg-secondary p-4">
      <div className="mb-3 text-xs text-gray-500">모델 상태</div>
      <div className="space-y-3">
        {/* Kalman */}
        <div>
          <div className="text-xs font-medium text-accent-cyan">Kalman Filter</div>
          <div className="mt-1 grid grid-cols-2 gap-1 text-xs">
            <span className="text-gray-500">gain:</span>
            <span className="font-mono text-white">0.032</span>
            <span className="text-gray-500">innovation:</span>
            <span className="font-mono text-white">-1.2</span>
            <span className="text-gray-500">est. error:</span>
            <span className="font-mono text-white">0.0012</span>
          </div>
        </div>
        {/* OU */}
        <div>
          <div className="text-xs font-medium text-accent-cyan">Ornstein-Uhlenbeck</div>
          <div className="mt-1 grid grid-cols-2 gap-1 text-xs">
            <span className="text-gray-500">kappa (κ):</span>
            <span className="font-mono text-white">0.045</span>
            <span className="text-gray-500">half-life:</span>
            <span className="font-mono text-white">15.4s</span>
            <span className="text-gray-500">z-score:</span>
            <span className="font-mono text-loss">-1.82</span>
          </div>
        </div>
        {/* GARCH */}
        <div>
          <div className="text-xs font-medium text-accent-cyan">GARCH</div>
          <div className="mt-1 grid grid-cols-2 gap-1 text-xs">
            <span className="text-gray-500">volatility:</span>
            <span className="font-mono text-white">2.05%</span>
            <span className="text-gray-500">persistence:</span>
            <span className="font-mono text-white">0.96</span>
          </div>
        </div>
      </div>
    </div>
  );
}
