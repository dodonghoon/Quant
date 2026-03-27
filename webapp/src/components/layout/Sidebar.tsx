'use client';

import { useState } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import {
  LayoutDashboard,
  BarChart3,
  Brain,
  Zap,
  ShieldAlert,
  FlaskConical,
  Settings,
  ScrollText,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react';

const navItems = [
  { href: '/', label: '대시보드', icon: LayoutDashboard },
  { href: '/market', label: '시장 데이터', icon: BarChart3 },
  { href: '/strategy', label: '전략', icon: Brain },
  { href: '/execution', label: '주문 집행', icon: Zap },
  { href: '/risk', label: '리스크', icon: ShieldAlert },
  { href: '/research', label: '리서치', icon: FlaskConical },
  { href: '/settings', label: '설정', icon: Settings },
  { href: '/logs', label: '감사 로그', icon: ScrollText },
];

export default function Sidebar() {
  const [collapsed, setCollapsed] = useState(false);
  const pathname = usePathname();

  return (
    <aside
      className={`flex flex-col border-r border-gray-800 bg-bg-secondary transition-all duration-200 ${
        collapsed ? 'w-16' : 'w-56'
      }`}
    >
      {/* 로고 */}
      <div className="flex h-14 items-center justify-between border-b border-gray-800 px-4">
        {!collapsed && (
          <span className="text-sm font-bold text-accent-blue">QUANT</span>
        )}
        <button
          onClick={() => setCollapsed(!collapsed)}
          className="rounded p-1 text-gray-400 hover:bg-bg-tertiary hover:text-white"
        >
          {collapsed ? <ChevronRight size={16} /> : <ChevronLeft size={16} />}
        </button>
      </div>

      {/* 네비게이션 */}
      <nav className="flex-1 space-y-1 p-2">
        {navItems.map(({ href, label, icon: Icon }) => {
          const isActive =
            href === '/' ? pathname === '/' : pathname.startsWith(href);
          return (
            <Link
              key={href}
              href={href}
              className={`flex items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors ${
                isActive
                  ? 'bg-accent-blue/10 text-accent-blue'
                  : 'text-gray-400 hover:bg-bg-tertiary hover:text-white'
              }`}
              title={collapsed ? label : undefined}
            >
              <Icon size={18} />
              {!collapsed && <span>{label}</span>}
            </Link>
          );
        })}
      </nav>

      {/* 버전 */}
      {!collapsed && (
        <div className="border-t border-gray-800 p-4 text-xs text-gray-600">
          v0.1.0
        </div>
      )}
    </aside>
  );
}
