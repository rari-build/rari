import { createElement } from 'react'
import { describe, expect, it } from 'vite-plus/test'
import { ImageResponse } from '../../../packages/rari/src/og/image-response'

describe('image response', () => {
  it('serializes style objects into toJSON props', () => {
    const response = new ImageResponse(
      createElement(
        'div',
        {
          style: {
            display: 'flex',
            flexDirection: 'column',
            background: '#0d1117',
            width: '100%',
          },
        },
        createElement('div', { style: { fontSize: 48, color: '#f0f6fc' } }, 'Hello'),
      ),
    )

    expect(response.toJSON()).toMatchObject({
      element: {
        props: {
          style: {
            display: 'flex',
            flexDirection: 'column',
            background: '#0d1117',
            width: '100%',
          },
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
})
