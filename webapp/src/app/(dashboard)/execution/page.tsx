"use client";

import { ArrowRight, BarChart3, Clock, Zap } from "lucide-react";
import Link from "next/link";

export default function ExecutionPage() {
  const stats = [
    {
      label: "Signals",
      value: "342",
      icon: Zap,
      color: "text-accent-cyan",
    },
    {
      label: "Orders",
      value: "128",
      icon: BarChart3,
      color: "text-accent-blue",
    },
    {
      label: "Fills",
      value: "115",
      icon: Clock,
      color: "text-profit",
    },
    {
      label: "Avg Latency",
      value: "45ms",
      icon: ArrowRight,
      color: "text-accent-cyan",
    },
  ];

  const sections = [
    {
      title: "Orders",
      description: "View active and completed orders",
      href: "/execution/orders",
    },
    {
      title: "Fills",
      description: "Review executed fills and trade history",
      href: "/execution/fills",
    },
  ];

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold mb-2">Execution</h1>
        <p className="text-gray-400">Real-time order and execution monitoring</p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {stats.map((stat) => {
          const Icon = stat.icon;
          return (
            <div
              key={stat.label}
              className="bg-bg-secondary rounded-lg p-6 border border-gray-700"
            >
              <div className="flex items-start justify-between">
                <div>
                  <p className="text-gray-400 text-sm mb-2">{stat.label}</p>
                  <p className="text-2xl font-bold">{stat.value}</p>
                </div>
                <Icon className={`w-5 h-5 ${stat.color}`} />
              </div>
            </div>
          );
        })}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {sections.map((section) => (
          <Link
            key={section.href}
            href={section.href}
            className="bg-bg-secondary rounded-lg p-6 border border-gray-700 hover:border-accent-cyan transition-colors group"
          >
            <h3 className="text-lg font-semibold mb-2 group-hover:text-accent-cyan transition-colors">
              {section.title}
            </h3>
            <p className="text-gray-400 text-sm mb-4">{section.description}</p>
            <div className="flex items-center text-accent-cyan text-sm">
              <span>View details</span>
              <ArrowRight className="w-4 h-4 ml-2" />
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}
