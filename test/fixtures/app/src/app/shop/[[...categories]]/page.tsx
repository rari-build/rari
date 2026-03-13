import type { Metadata, PageProps } from 'rari'

export default async function ShopPage({ params }: PageProps) {
  const { categories } = params
  const categoriesArray = categories ? (Array.isArray(categories) ? categories : [categories]) : []
  const hasCategories = categoriesArray.length > 0

  return (
    <div>
      <h1>
        Shop
        {hasCategories ? `: ${categoriesArray.join(' > ')}` : ''}
      </h1>
      <p>This is an optional catch-all route.</p>
      {hasCategories
        ? (
            <div data-testid="categories" data-categories={JSON.stringify(categoriesArray)}>
              {categoriesArray.map((category, i) => (
                <span key={i} data-testid={`category-${i}`}>{category}</span>
              ))}
            </div>
          )
        : (
            <div data-testid="no-categories">All Products</div>
          )}
      <div data-testid="categories-length">{String(categoriesArray.length)}</div>
    </div>
  )
}

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { categories } = params
  const categoriesArray = categories ? (Array.isArray(categories) ? categories : [categories]) : []
  const title = categoriesArray.length > 0 ? `Shop: ${categoriesArray.join(' > ')}` : 'Shop'

  return {
    title,
    description: 'Shop products',
  }
}
