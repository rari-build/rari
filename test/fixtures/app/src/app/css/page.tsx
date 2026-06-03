import styles from './styles.module.css'

export default function CssPage() {
  return (
    <div>
      <h1>CSS Module Test</h1>
      <p data-testid="module-css-text" className={styles.red}>styled text</p>
    </div>
  )
}
