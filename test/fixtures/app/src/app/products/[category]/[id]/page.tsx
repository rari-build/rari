import type { Metadata, PageProps } from 'rari'

export default async function ProductPage({ params }: PageProps) {
  const { category, id } = params

  return (
    <div>
      <h1>
        Product
        {' '}
        {id}
      </h1>
      <p>
        Category:
        {' '}
        {category}
      </p>
      <div data-testid="category-value">{category}</div>
      <div data-testid="id-value">{id}</div>
      <a href="/products">Back to Products</a>
    </div>
  )
}

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { category, id } = params

  return {
    title: `${category} - Product ${id}`,
    description: `Product ${id} in ${category} category`,
  }
}
