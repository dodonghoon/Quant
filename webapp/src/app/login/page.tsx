'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { Lock, User, AlertCircle } from 'lucide-react';
import { auth, setTokens } from '@/lib/api';
import { useTradingStore } from '@/stores/tradingStore';

export default function LoginPage() {
  const router = useRouter();
  const setAuth = useTradingStore((s) => s.setAuth);
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  async function handleLogin(e: React.FormEvent) {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      const data = await auth.login(username, password);
      setTokens(data.access_token, data.refresh_token);
      setAuth({ token: data.access_token, role: data.role });
      router.push('/');
    } catch (err: any) {
      setError(err?.body?.message || '로그인에 실패했습니다');
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-bg-primary">
      <div className="w-full max-w-sm rounded-lg border border-gray-800 bg-bg-secondary p-8 shadow-xl">
        <div className="mb-8 text-center">
          <h1 className="text-2xl font-bold text-accent-blue">QUANT</h1>
          <p className="mt-1 text-sm text-gray-500">Trading Dashboard</p>
        </div>

        <form onSubmit={handleLogin} className="space-y-4">
          <div>
            <label className="mb-1 block text-sm text-gray-400">사용자명</label>
            <div className="flex items-center gap-2 rounded-md border border-gray-700 bg-bg-tertiary px-3 py-2">
              <User size={16} className="text-gray-500" />
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="flex-1 bg-transparent text-sm text-white outline-none placeholder:text-gray-600"
                placeholder="admin"
                required
              />
            </div>
          </div>

          <div>
            <label className="mb-1 block text-sm text-gray-400">비밀번호</label>
            <div className="flex items-center gap-2 rounded-md border border-gray-700 bg-bg-tertiary px-3 py-2">
              <Lock size={16} className="text-gray-500" />
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="flex-1 bg-transparent text-sm text-white outline-none placeholder:text-gray-600"
                placeholder="********"
                required
              />
            </div>
          </div>

          {error && (
            <div className="flex items-center gap-2 rounded-md bg-red-600/10 px-3 py-2 text-sm text-red-400">
              <AlertCircle size={14} />
              {error}
            </div>
          )}

          <button
            type="submit"
            disabled={loading}
            className="w-full rounded-md bg-accent-blue py-2.5 text-sm font-semibold text-white transition-colors hover:bg-blue-600 disabled:opacity-50"
          >
            {loading ? '로그인 중...' : '로그인'}
          </button>
        </form>

        <p className="mt-6 text-center text-xs text-gray-600">
          기본 계정: admin / admin123
        </p>
      </div>
    </div>
  );
}
