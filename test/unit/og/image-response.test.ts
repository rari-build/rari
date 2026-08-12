import { createElement } from 'react'
import { describe, expect, it } from 'vite-plus/test'
import { ImageResponse } from '../../../packages/rari/src/og/image-response'

describe('image response', () => {
  it('serializes style objects into toJSON props', () => {
    const response = new ImageResponse(
      createElement(
        'div',
        {
          'style': {
            display: 'flex',
            flexDirection: 'column',
            background: '#0d1117',
            width: '100%',
          },
          'data-values': ['ok', 1, true, () => 'skip', Symbol('skip')],
        },
        createElement('div', { style: { fontSize: 48, color: '#f0f6fc' } }, 'Hello'),
      ),
    )

    expect(response.toJSON()).toMatchObject({
      element: {
        props: {
          'style': {
            display: 'flex',
            flexDirection: 'column',
            background: '#0d1117',
            width: '100%',
          },
          'data-values': ['ok', 1, true],
        },
        children: [
          {
            props: {
              style: {
                fontSize: 48,
                color: '#f0f6fc',
              },
            },
          },
        ],
      },
    })
  })

  it('omits cyclic object and array references', () => {
    const cyclicObject: Record<string, unknown> = { color: '#fff' }
    cyclicObject.self = cyclicObject

    const cyclicArray: unknown[] = ['ok']
    cyclicArray.push(cyclicArray)

    const shared = { tone: 'muted' }
    const response = new ImageResponse(
      createElement('div', {
        'style': cyclicObject,
        'data-items': cyclicArray,
        'data-a': shared,
        'data-b': shared,
      }),
    )

    expect(response.toJSON()).toMatchObject({
      element: {
        props: {
          'style': { color: '#fff' },
          'data-items': ['ok'],
          'data-a': { tone: 'muted' },
          'data-b': { tone: 'muted' },
        },
      },
    })
  })
})
