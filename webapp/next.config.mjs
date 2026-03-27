/** @type {import('next').NextConfig} */
const nextConfig = {
  // Rust Axum 백엔드 프록시
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: 'http://127.0.0.1:8080/api/:path*',
      },
      {
        source: '/ws/:path*',
        destination: 'http://127.0.0.1:8080/ws/:path*',
      },
    ];
  },
  // 성능 최적화
  reactStrictMode: true,
  poweredByHeader: false,
};

export default nextConfig;
