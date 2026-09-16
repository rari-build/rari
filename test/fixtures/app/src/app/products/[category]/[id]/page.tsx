import type { Metadata, PageProps } from 'rari'

export default function ProductPage({ params }: PageProps<'/products/[category]/[id]'>) {
  const { category, id } = params

  return (
    <div>
      <h1>Product {id}</h1>
      <p>Category: {category}</p>
      <div data-testid="category-value">{category}</div>
      <div data-testid="id-value">{id}</div>
      <a href="/products">Back to Products</a>
    </div>
  )
}

export function generateMetadata({ params }: PageProps<'/products/[category]/[id]'>): Metadata {
  const { category, id } = params

  return {
    title: `${category} - Product ${id}`,
    description: `Product ${id} in ${category} category`,
  }
}
