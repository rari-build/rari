import * as stylex from '@stylexjs/stylex'

const styles = stylex.create({
  text: {
    color: 'rgb(0, 128, 0)',
  },
})

export default function StylexPage() {
  return (
    <div>
      <h1>StyleX Test</h1>
      <p data-testid="stylex-text" {...stylex.props(styles.text)}>
        stylex text
      </p>
    </div>
  )
}
