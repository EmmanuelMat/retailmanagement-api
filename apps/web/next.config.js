/** @type {import('next').NextConfig} */
const nextConfig = {
  transpilePackages: ["@repo/ui", "@repo/api-client"],
  experimental: {
    typedRoutes: true,
  },
};

export default nextConfig;
