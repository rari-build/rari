import { getTodos } from '../../actions/todo-actions'
import ProgressiveFormExample from '../../components/ProgressiveFormExample'
import TodoAppWithActions from '../../components/TodoAppWithActions'

export default async function ActionsPage() {
  const initialTodos = await getTodos()

  return (
    <div style={{
      background: 'white',
      borderRadius: '12px',
      padding: '3rem',
      boxShadow: '0 10px 40px rgba(0, 0, 0, 0.1)',
    }}
    >
      <header style={{
        textAlign: 'center',
        marginBottom: '3rem',
        paddingBottom: '2rem',
        borderBottom: '2px solid #e2e8f0',
      }}
      >
        <h1 style={{
          fontSize: '3rem',
          marginBottom: '1rem',
          background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
          WebkitBackgroundClip: 'text',
          WebkitTextFillColor: 'transparent',
          backgroundClip: 'text',
        }}
        >
          React Server Actions Demo
        </h1>
        <p style={{
          fontSize: '1.2rem',
          color: '#666',
          maxWidth: '600px',
          margin: '0 auto',
          lineHeight: '1.6',
        }}
        >
          This page demonstrates React Server Actions working with Rari.
          All patterns follow React's official server function specifications.
        </p>
      </header>

      <main style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '2rem',
      }}
      >
        <section>
          <h2 style={{
            fontSize: '2rem',
            color: '#667eea',
            marginBottom: '1.5rem',
          }}
          >
            Interactive Todo App
          </h2>

          <TodoAppWithActions initialTodos={initialTodos} />
        </section>

        <section>
          <h2 style={{
            fontSize: '2rem',
            color: '#667eea',
            marginBottom: '1.5rem',
          }}
          >
            Progressive Enhancement
          </h2>

          <ProgressiveFormExample />
        </section>

        <section style={{
          background: 'white',
          padding: '2rem',
          borderRadius: '1rem',
          boxShadow: '0 10px 40px rgba(0, 0, 0, 0.1)',
        }}
        >
          <h2 style={{
            fontSize: '2rem',
            color: '#667eea',
            marginBottom: '1.5rem',
          }}
          >
            Server Action Patterns Demonstrated
          </h2>

          <div style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))',
            gap: '1.5rem',
          }}
          >
            <div style={{
              padding: '1.5rem',
              background: '#f8fafc',
              borderRadius: '0.5rem',
              border: '1px solid #e2e8f0',
            }}
            >
              <h3 style={{
                color: '#10b981',
                marginBottom: '0.5rem',
                fontSize: '1.2rem',
              }}
              >
                ✅ useActionState Hook
              </h3>
              <p style={{
                color: '#666',
                margin: 0,
                lineHeight: '1.5',
              }}
              >
                Manage server action state with pending states and error handling.
              </p>
            </div>

            <div style={{
              padding: '1.5rem',
              background: '#f8fafc',
              borderRadius: '0.5rem',
              border: '1px solid #e2e8f0',
            }}
            >
              <h3 style={{
                color: '#10b981',
                marginBottom: '0.5rem',
                fontSize: '1.2rem',
              }}
              >
                ✅ useTransition Hook
              </h3>
              <p style={{
                color: '#666',
                margin: 0,
                lineHeight: '1.5',
              }}
              >
                Track pending states across multiple actions for better UX.
              </p>
            </div>

            <div style={{
              padding: '1.5rem',
              background: '#f8fafc',
              borderRadius: '0.5rem',
              border: '1px solid #e2e8f0',
            }}
            >
              <h3 style={{
                color: '#10b981',
                marginBottom: '0.5rem',
                fontSize: '1.2rem',
              }}
              >
                ✅ Form Actions
              </h3>
              <p style={{
                color: '#666',
                margin: 0,
                lineHeight: '1.5',
              }}
              >
                Server functions that work with HTML forms and FormData.
              </p>
            </div>

            <div style={{
              padding: '1.5rem',
              background: '#f8fafc',
              borderRadius: '0.5rem',
              border: '1px solid #e2e8f0',
            }}
            >
              <h3 style={{
                color: '#10b981',
                marginBottom: '0.5rem',
                fontSize: '1.2rem',
              }}
              >
                ✅ Progressive Enhancement
              </h3>
              <p style={{
                color: '#666',
                margin: 0,
                lineHeight: '1.5',
              }}
              >
                Forms that work without JavaScript and enhance with it.
              </p>
            </div>

            <div style={{
              padding: '1.5rem',
              background: '#f8fafc',
              borderRadius: '0.5rem',
              border: '1px solid #e2e8f0',
            }}
            >
              <h3 style={{
                color: '#10b981',
                marginBottom: '0.5rem',
                fontSize: '1.2rem',
              }}
              >
                ✅ Error Handling
              </h3>
              <p style={{
                color: '#666',
                margin: 0,
                lineHeight: '1.5',
              }}
              >
                Proper error states and user feedback for failed actions.
              </p>
            </div>

            <div style={{
              padding: '1.5rem',
              background: '#f8fafc',
              borderRadius: '0.5rem',
              border: '1px solid #e2e8f0',
            }}
            >
              <h3 style={{
                color: '#10b981',
                marginBottom: '0.5rem',
                fontSize: '1.2rem',
              }}
              >
                ✅ Redirects
              </h3>
              <p style={{
                color: '#666',
                margin: 0,
                lineHeight: '1.5',
              }}
              >
                Server actions can redirect after successful completion.
              </p>
            </div>
          </div>
        </section>

        <section style={{
          background: 'white',
          padding: '2rem',
          borderRadius: '1rem',
          boxShadow: '0 10px 40px rgba(0, 0, 0, 0.1)',
        }}
        >
          <h2 style={{
            fontSize: '2rem',
            color: '#667eea',
            marginBottom: '1.5rem',
          }}
          >
            Technical Implementation
          </h2>

          <div style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fit, minmax(250px, 1fr))',
            gap: '1.5rem',
          }}
          >
            <div style={{
              padding: '1.5rem',
              background: '#fef5e7',
              borderRadius: '0.5rem',
              border: '1px solid #f9e79f',
            }}
            >
              <h4 style={{
                color: '#d68910',
                marginBottom: '0.5rem',
                fontSize: '1.1rem',
              }}
              >
                Server Functions
              </h4>
              <p style={{
                color: '#666',
                margin: 0,
                lineHeight: '1.5',
              }}
              >
                Functions marked with
                {' '}
                <code style={{
                  background: '#e2e8f0',
                  padding: '0.125rem 0.25rem',
                  borderRadius: '0.25rem',
                  fontFamily: 'monospace',
                  fontSize: '0.9em',
                }}
                >
                  'use server'
                </code>
                {' '}
                are automatically transformed into callable references.
              </p>
            </div>

            <div style={{
              padding: '1.5rem',
              background: '#fef5e7',
              borderRadius: '0.5rem',
              border: '1px solid #f9e79f',
            }}
            >
              <h4 style={{
                color: '#d68910',
                marginBottom: '0.5rem',
                fontSize: '1.1rem',
              }}
              >
                HTTP Endpoints
              </h4>
              <p style={{
                color: '#666',
                margin: 0,
                lineHeight: '1.5',
              }}
              >
                Server actions are called via
                {' '}
                <code style={{
                  background: '#e2e8f0',
                  padding: '0.125rem 0.25rem',
                  borderRadius: '0.25rem',
                  fontFamily: 'monospace',
                  fontSize: '0.9em',
                }}
                >
                  POST /api/rsc/action
                </code>
                {' '}
                with JSON payloads.
              </p>
            </div>

            <div style={{
              padding: '1.5rem',
              background: '#fef5e7',
              borderRadius: '0.5rem',
              border: '1px solid #f9e79f',
            }}
            >
              <h4 style={{
                color: '#d68910',
                marginBottom: '0.5rem',
                fontSize: '1.1rem',
              }}
              >
                Form Enhancement
              </h4>
              <p style={{
                color: '#666',
                margin: 0,
                lineHeight: '1.5',
              }}
              >
                Forms can post to
                {' '}
                <code style={{
                  background: '#e2e8f0',
                  padding: '0.125rem 0.25rem',
                  borderRadius: '0.25rem',
                  fontFamily: 'monospace',
                  fontSize: '0.9em',
                }}
                >
                  /api/rsc/form-action
                </code>
                {' '}
                for progressive enhancement.
              </p>
            </div>

            <div style={{
              padding: '1.5rem',
              background: '#fef5e7',
              borderRadius: '0.5rem',
              border: '1px solid #f9e79f',
            }}
            >
              <h4 style={{
                color: '#d68910',
                marginBottom: '0.5rem',
                fontSize: '1.1rem',
              }}
              >
                Wire Format
              </h4>
              <p style={{
                color: '#666',
                margin: 0,
                lineHeight: '1.5',
              }}
              >
                Actions return JSON responses that can include redirects and error states.
              </p>
            </div>
          </div>
        </section>
      </main>
    </div>
  )
}

export const metadata = {
  title: 'Server Actions Demo | Rari App Router',
  description: 'Demonstration of React Server Actions with Rari framework',
}
