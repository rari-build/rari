import type { MouseEvent, ReactNode } from 'react'
import type { LinkProps, NavigationOptions } from './types'
import React, { useCallback } from 'react'
import { useRouter } from './router'

const DEFAULT_OPTIONS: NavigationOptions = {}

const DEFAULT_LABELS = {
  first: 'First',
  previous: 'Previous',
  next: 'Next',
  last: 'Last',
}

export function Link({
  ref,
  to,
  options = DEFAULT_OPTIONS,
  children,
  className,
  onClick,
  replace = false,
  ...props
}: LinkProps & { ref?: React.RefObject<HTMLAnchorElement | null> }) {
  const { navigate } = useRouter()

  const handleClick = useCallback(
    (event: MouseEvent<HTMLAnchorElement>) => {
      if (onClick) {
        onClick(event)
      }

      if (
        event.defaultPrevented
        || event.button !== 0
        || event.metaKey
        || event.ctrlKey
        || event.shiftKey
        || event.altKey
      ) {
        return
      }

      event.preventDefault()

      navigate(to, { replace, ...options })
    },
    [navigate, to, replace, options, onClick],
  )

  return (
    <a
      ref={ref}
      href={to}
      className={className}
      onClick={handleClick}
      {...props}
    >
      {children}
    </a>
  )
}

Link.displayName = 'Link'

export interface NavLinkProps extends Omit<LinkProps, 'activeClassName'> {
  activeClassName?: string
  activeStyle?: React.CSSProperties
  exact?: boolean
  isActive?: (pathname: string, to: string) => boolean
  disabled?: boolean
}

export function NavLink({
  ref,
  to,
  children,
  className,
  activeClassName,
  activeStyle,
  exact = false,
  isActive: customIsActive,
  disabled = false,
  style,
  ...props
}: NavLinkProps & { ref?: React.RefObject<HTMLAnchorElement | null> }) {
  const { isActive: routerIsActive, currentRoute } = useRouter()

  const isLinkActive = useCallback(() => {
    if (disabled)
      return false

    if (customIsActive) {
      return customIsActive(currentRoute?.pathname || '/', to)
    }

    return routerIsActive(to, exact)
  }, [
    customIsActive,
    currentRoute?.pathname,
    to,
    routerIsActive,
    exact,
    disabled,
  ])

  const active = isLinkActive()

  const combinedClassName = [className, active && activeClassName]
    .filter(Boolean)
    .join(' ')

  const combinedStyle = {
    ...style,
    ...(active && activeStyle),
  }

  return (
    <Link
      ref={ref}
      to={to}
      className={combinedClassName}
      style={combinedStyle}
      {...props}
    >
      {typeof children === 'function'
        ? children({ isActive: active })
        : children}
    </Link>
  )
}

NavLink.displayName = 'NavLink'

export interface FormProps extends React.FormHTMLAttributes<HTMLFormElement> {
  action?: string
  method?: 'get' | 'post' | 'put' | 'delete' | 'patch'
  replace?: boolean
  options?: NavigationOptions
}

export function Form({
  ref,
  action,
  method = 'post',
  replace = false,
  options = DEFAULT_OPTIONS,
  onSubmit,
  children,
  ...props
}: FormProps & { ref?: React.RefObject<HTMLFormElement | null> }) {
  const { navigate } = useRouter()

  const handleSubmit = useCallback(
    (event: React.FormEvent<HTMLFormElement>) => {
      if (onSubmit) {
        onSubmit(event)
      }

      if (method === 'get' && action && !event.defaultPrevented) {
        event.preventDefault()

        const formData = new FormData(event.currentTarget)
        const searchParams = new URLSearchParams(formData as any)
        const url = `${action}?${searchParams.toString()}`

        navigate(url, { replace, ...options })
      }
    },
    [navigate, action, method, replace, options, onSubmit],
  )

  return (
    <form
      ref={ref}
      action={action}
      method={method}
      onSubmit={handleSubmit}
      {...props}
    >
      {children}
    </form>
  )
}

Form.displayName = 'Form'

export interface BreadcrumbProps {
  items: Array<{
    label: ReactNode
    href?: string
    active?: boolean
  }>
  separator?: ReactNode
  className?: string
  itemClassName?: string
  separatorClassName?: string
  activeClassName?: string
}

export function Breadcrumb({
  items,
  separator = '/',
  className,
  itemClassName,
  separatorClassName,
  activeClassName,
}: BreadcrumbProps) {
  return (
    <nav className={className} aria-label="Breadcrumb">
      <ol className="flex items-center space-x-2">
        {items.map((item, index) => (
          <li key={index} className="flex items-center">
            {index > 0 && (
              <span className={separatorClassName} aria-hidden="true">
                {separator}
              </span>
            )}
            {item.href && !item.active
              ? (
                  <Link
                    to={item.href}
                    className={[itemClassName, item.active && activeClassName]
                      .filter(Boolean)
                      .join(' ')}
                  >
                    {item.label}
                  </Link>
                )
              : (
                  <span
                    className={[itemClassName, item.active && activeClassName]
                      .filter(Boolean)
                      .join(' ')}
                    aria-current={item.active ? 'page' : undefined}
                  >
                    {item.label}
                  </span>
                )}
          </li>
        ))}
      </ol>
    </nav>
  )
}

export interface BackButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  fallback?: string
  children?: ReactNode
}

export function BackButton({
  ref,
  fallback = '/',
  children = 'Back',
  onClick,
  ...props
}: BackButtonProps & { ref?: React.RefObject<HTMLButtonElement | null> }) {
  const { back, navigate } = useRouter()

  const handleClick = useCallback(
    (event: React.MouseEvent<HTMLButtonElement>) => {
      if (onClick) {
        onClick(event)
      }

      if (!event.defaultPrevented) {
        if (window.history.length > 1) {
          back()
        }
        else {
          navigate(fallback)
        }
      }
    },
    [back, navigate, fallback, onClick],
  )

  return (
    <button type="button" ref={ref} onClick={handleClick} {...props}>
      {children}
    </button>
  )
}

BackButton.displayName = 'BackButton'

export interface NavigationMenuProps {
  items: Array<{
    label: ReactNode
    href: string
    active?: boolean
    disabled?: boolean
    children?: Array<{
      label: ReactNode
      href: string
      active?: boolean
      disabled?: boolean
    }>
  }>
  className?: string
  itemClassName?: string
  activeClassName?: string
  disabledClassName?: string
  exact?: boolean
}

export function NavigationMenu({
  items,
  className,
  itemClassName,
  activeClassName,
  disabledClassName,
  exact = false,
}: NavigationMenuProps) {
  return (
    <nav className={className}>
      <ul>
        {items.map((item, index) => (
          <li key={index}>
            {item.disabled
              ? (
                  <span
                    className={[itemClassName, disabledClassName]
                      .filter(Boolean)
                      .join(' ')}
                  >
                    {item.label}
                  </span>
                )
              : (
                  <NavLink
                    to={item.href}
                    className={itemClassName}
                    activeClassName={activeClassName}
                    exact={exact}
                  >
                    {item.label}
                  </NavLink>
                )}
            {item.children && (
              <ul>
                {item.children.map((child, childIndex) => (
                  <li key={childIndex}>
                    {child.disabled
                      ? (
                          <span
                            className={[itemClassName, disabledClassName]
                              .filter(Boolean)
                              .join(' ')}
                          >
                            {child.label}
                          </span>
                        )
                      : (
                          <NavLink
                            to={child.href}
                            className={itemClassName}
                            activeClassName={activeClassName}
                            exact={exact}
                          >
                            {child.label}
                          </NavLink>
                        )}
                  </li>
                ))}
              </ul>
            )}
          </li>
        ))}
      </ul>
    </nav>
  )
}

export interface PaginationProps {
  currentPage: number
  totalPages: number
  getPageUrl: (page: number) => string
  maxPages?: number
  className?: string
  pageClassName?: string
  activeClassName?: string
  disabledClassName?: string
  showFirstLast?: boolean
  showPrevNext?: boolean
  labels?: {
    first?: ReactNode
    previous?: ReactNode
    next?: ReactNode
    last?: ReactNode
  }
}

export function Pagination({
  currentPage,
  totalPages,
  getPageUrl,
  maxPages = 10,
  className,
  pageClassName,
  activeClassName,
  disabledClassName,
  showFirstLast = true,
  showPrevNext = true,
  labels = DEFAULT_LABELS,
}: PaginationProps) {
  const startPage = Math.max(1, currentPage - Math.floor(maxPages / 2))
  const endPage = Math.min(totalPages, startPage + maxPages - 1)
  const pages = Array.from(
    { length: endPage - startPage + 1 },
    (_, i) => startPage + i,
  )

  const canGoPrevious = currentPage > 1
  const canGoNext = currentPage < totalPages

  return (
    <nav className={className} aria-label="Pagination">
      <ul className="flex items-center space-x-1">
        {showFirstLast && (
          <li>
            {canGoPrevious
              ? (
                  <Link to={getPageUrl(1)} className={pageClassName}>
                    {labels.first}
                  </Link>
                )
              : (
                  <span
                    className={[pageClassName, disabledClassName]
                      .filter(Boolean)
                      .join(' ')}
                  >
                    {labels.first}
                  </span>
                )}
          </li>
        )}

        {showPrevNext && (
          <li>
            {canGoPrevious
              ? (
                  <Link to={getPageUrl(currentPage - 1)} className={pageClassName}>
                    {labels.previous}
                  </Link>
                )
              : (
                  <span
                    className={[pageClassName, disabledClassName]
                      .filter(Boolean)
                      .join(' ')}
                  >
                    {labels.previous}
                  </span>
                )}
          </li>
        )}

        {pages.map(page => (
          <li key={page}>
            {page === currentPage
              ? (
                  <span
                    className={[pageClassName, activeClassName]
                      .filter(Boolean)
                      .join(' ')}
                    aria-current="page"
                  >
                    {page}
                  </span>
                )
              : (
                  <Link to={getPageUrl(page)} className={pageClassName}>
                    {page}
                  </Link>
                )}
          </li>
        ))}

        {showPrevNext && (
          <li>
            {canGoNext
              ? (
                  <Link to={getPageUrl(currentPage + 1)} className={pageClassName}>
                    {labels.next}
                  </Link>
                )
              : (
                  <span
                    className={[pageClassName, disabledClassName]
                      .filter(Boolean)
                      .join(' ')}
                  >
                    {labels.next}
                  </span>
                )}
          </li>
        )}

        {showFirstLast && (
          <li>
            {canGoNext
              ? (
                  <Link to={getPageUrl(totalPages)} className={pageClassName}>
                    {labels.last}
                  </Link>
                )
              : (
                  <span
                    className={[pageClassName, disabledClassName]
                      .filter(Boolean)
                      .join(' ')}
                  >
                    {labels.last}
                  </span>
                )}
          </li>
        )}
      </ul>
    </nav>
  )
}

export default {
  Link,
  NavLink,
  Form,
  Breadcrumb,
  BackButton,
  NavigationMenu,
  Pagination,
}
