import { useState, useEffect } from "react";
import { RouterProvider, useRouter } from "rari/client";
import { routes } from "../.rari/routes";
import Layout from "./components/Layout";

function getMetaDescription(pathname: string): string {
  switch (pathname) {
    case "/":
      return "Rari is a performance-first React framework powered by Rust. Build web applications with React Server Components, zero-config setup, and runtime-accelerated rendering infrastructure.";
    case "/getting-started":
      return "Learn how to build your first Rari application. Step-by-step guide to creating high-performance React apps with Rust-powered server components and zero-config setup.";
    default:
      return "Rari - Performance-first React framework powered by Rust for building web applications.";
  }
}

function NotFoundPage() {
  return (
    <div className="min-h-screen bg-[#0d1117] text-[#f0f6fc] flex items-center justify-center">
      <div className="text-center">
        <h1 className="text-4xl font-bold mb-4">404 - Page Not Found</h1>
        <p className="text-gray-400 mb-8">
          The page you're looking for doesn't exist.
        </p>
        <a
          href="/"
          className="bg-[#fd7e14] hover:bg-[#e8590c] text-white px-6 py-3 rounded-lg font-semibold transition-colors"
        >
          Go Home
        </a>
      </div>
    </div>
  );
}

const routerConfig = {
  notFoundComponent: NotFoundPage,
  basePath: "",
  useHash: false,
  caseSensitive: false,
};

function App() {
  return (
    <RouterProvider config={routerConfig} routes={routes}>
      <Routes />
    </RouterProvider>
  );
}

function Routes() {
  const { currentRoute } = useRouter();
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) {
    return null;
  }

  if (!currentRoute) {
    return <NotFoundPage />;
  }

  const Component = currentRoute.route.component;
  const { params, searchParams } = currentRoute;

  if (!Component) {
    return <NotFoundPage />;
  }

  return (
    <Layout
      currentPage={getPageFromPath(currentRoute.pathname)}
      metaDescription={getMetaDescription(currentRoute.pathname)}
    >
      <Component
        params={params}
        searchParams={searchParams}
        meta={currentRoute.route.meta}
      />
    </Layout>
  );
}

function getPageFromPath(pathname: string): string {
  if (pathname === "/") return "home";
  return pathname.slice(1);
}

export default App;
