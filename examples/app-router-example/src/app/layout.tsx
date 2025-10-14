import type { LayoutProps } from 'rari/client'

export default function RootLayout({ children }: LayoutProps) {
  return (
    <html lang="en">
      <head>
        <meta charSet="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>Rari App Router Example</title>
        <style>
          {`
          * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
          }

          body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen',
              'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue',
              sans-serif;
            -webkit-font-smoothing: antialiased;
            -moz-osx-font-smoothing: grayscale;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            color: #333;
          }

          nav {
            background: rgba(255, 255, 255, 0.95);
            padding: 1rem 2rem;
            box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
          }

          nav ul {
            list-style: none;
            display: flex;
            gap: 2rem;
          }

          nav a {
            color: #667eea;
            text-decoration: none;
            font-weight: 600;
            transition: color 0.2s;
          }

          nav a:hover {
            color: #764ba2;
          }

          main {
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem;
          }
        `}
        </style>
      </head>
      <body>
        <div id="root">
          <nav>
            <ul>
              <li><a href="/">Home</a></li>
              <li><a href="/about">About</a></li>
              <li><a href="/blog">Blog</a></li>
              <li><a href="/products">Products</a></li>
              <li><a href="/interactive">Interactive</a></li>
              <li><a href="/server-data">Server Data</a></li>
              <li><a href="/server-demo">Server Demo</a></li>
              <li><a href="/actions">Server Actions</a></li>
            </ul>
          </nav>
          <main>{children}</main>
        </div>
      </body>
    </html>
  )
}

export const metadata = {
  title: 'Rari App Router Example',
  description: 'Testing the new app router implementation',
}
