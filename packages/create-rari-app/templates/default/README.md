# {{PROJECT_NAME}}

> Runtime Accelerated Rendering Infrastructure

A React Server Components application powered by [rari](https://rari.build).

## Getting Started

```bash
# Install dependencies
{{INSTALL_COMMAND}}

# Start development server
{{PACKAGE_MANAGER}} run dev
```

Visit [http://localhost:5173](http://localhost:5173) to see your app.

## Features

- **App Router** — File-based routing with layouts, loading states, and error boundaries
- **React Server Components** — Server components by default, client components when you need them
- **Rust-powered Runtime** — HTTP server, RSC renderer, and routing written in Rust with embedded V8
- **Streaming SSR** — Progressive rendering with Suspense boundaries
- **Hot Module Reloading** — Instant feedback during development
- **TypeScript-first** — Full type safety across the server/client boundary
- **Zero Configuration** — Works out of the box with pre-built binaries
- **Cross-platform** — Supports macOS, Linux, and Windows

## Deploy to the Cloud

This rari application is pre-configured for cloud deployment.

### Railway

1. Push to GitHub:
```bash
git add .
git commit -m "Initial commit"
git push origin main
```

2. Deploy to Railway:
   - Go to [railway.app](https://railway.app)
   - Create new project → "Deploy from GitHub repo"
   - Select your repository
   - Click "Deploy Now"

3. Generate a domain:
   - In Railway dashboard → Settings → Networking
   - Click "Generate Domain"

**CLI Setup**

```bash
# Configure Railway deployment files
{{PACKAGE_MANAGER}} run deploy:railway

# Follow the instructions to deploy
```

### Render

1. Push to GitHub:
```bash
git add .
git commit -m "Initial commit"
git push origin main
```

2. Deploy to Render:
   - Go to [render.com](https://render.com)
   - Create a new "Web Service"
   - Connect your GitHub repository
   - Render auto-detects Node.js and uses `render.yaml`
   - Click "Create Web Service"

**CLI Setup**

```bash
# Configure Render deployment files
{{PACKAGE_MANAGER}} run deploy:render

# Follow the instructions to deploy
```

## Available Scripts

```bash
# Development
{{PACKAGE_MANAGER}} run dev             # Start development server
{{PACKAGE_MANAGER}} run build           # Build for production

# Production
{{PACKAGE_MANAGER}} start               # Start production server

# Deployment
{{PACKAGE_MANAGER}} run deploy:railway  # Set up Railway deployment
{{PACKAGE_MANAGER}} run deploy:render   # Set up Render deployment

# Code Quality
{{PACKAGE_MANAGER}} run typecheck       # Run TypeScript checks
```

## Learn More

- [rari Documentation](https://rari.build/docs)
- [Railway Documentation](https://docs.railway.app)
- [Render Documentation](https://render.com/docs)
- [React Server Components](https://react.dev/reference/react/use-server)

---

Built with [rari](https://rari.build)
