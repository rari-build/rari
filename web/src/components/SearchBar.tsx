'use client'

import type { SearchResult } from '@/actions/search'
import { useRouter } from 'rari/router'
import { useEffect, useMemo, useRef, useState, useTransition } from 'react'
import { createPortal } from 'react-dom'
import { searchDocumentation } from '@/actions/search'
import Close from './icons/Close'
import Search from './icons/Search'

const highlightRegex = /[.*+?^${}()|[\]\\]/g

function escapeRegex(str: string): string {
  return str.replace(highlightRegex, '\\$&')
}

export default function SearchBar() {
  const [isOpen, setIsOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [rawResults, setRawResults] = useState<SearchResult[]>([])
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [isPending, startTransition] = useTransition()
  const inputRef = useRef<HTMLInputElement>(null)
  const resultItemRef = useRef<(HTMLButtonElement | null)[]>([])
  const router = useRouter()
  const debounceRef = useRef<NodeJS.Timeout | undefined>(undefined)

  const results = useMemo(() => query.trim() ? rawResults : [], [query, rawResults])

  const queryRef = useRef(query)
  if (queryRef.current !== query) {
    queryRef.current = query
    if (selectedIndex !== 0)
      setSelectedIndex(0)
  }

  useEffect(() => {
    if (resultItemRef.current[selectedIndex]) {
      resultItemRef.current[selectedIndex]?.scrollIntoView({
        behavior: 'smooth',
        block: 'nearest',
      })
    }
  }, [selectedIndex])

  const handleClose = () => {
    setIsOpen(false)
    setQuery('')
    setRawResults([])
    setSelectedIndex(0)
  }

  useEffect(() => {
    if (!query.trim())
      return

    if (debounceRef.current)
      clearTimeout(debounceRef.current)

    debounceRef.current = setTimeout(() => {
      startTransition(async () => {
        const searchResults = await searchDocumentation(query)
        setRawResults(searchResults)
      })
    }, 150)

    return () => {
      if (debounceRef.current)
        clearTimeout(debounceRef.current)
    }
  }, [query])

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        setIsOpen(true)
      }
      if (e.key === 'Escape' && isOpen) {
        setIsOpen(false)
      }
      if (isOpen && results.length > 0) {
        if (e.key === 'ArrowDown') {
          e.preventDefault()
          setSelectedIndex(prev => (prev + 1) % results.length)
        }
        if (e.key === 'ArrowUp') {
          e.preventDefault()
          setSelectedIndex(prev => (prev - 1 + results.length) % results.length)
        }
        if (e.key === 'Enter') {
          e.preventDefault()
          const selected = results[selectedIndex]
          if (selected) {
            router.push(selected.href)
            handleClose()
          }
        }
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [isOpen, results, selectedIndex, router])

  useEffect(() => {
    if (isOpen && inputRef.current)
      inputRef.current.focus()
  }, [isOpen])

  const handleResultClick = (href: string) => {
    router.push(href)
    handleClose()
  }

  return (
    <>
      <button
        type="button"
        onClick={() => setIsOpen(true)}
        className="w-full flex items-center gap-2 pl-3 pr-3 py-1.5 bg-[#161b22] border border-[#30363d] rounded-md text-sm text-gray-500 hover:border-[#fd7e14]/50 hover:text-gray-400 transition-all group"
        aria-label="Open search"
      >
        <Search className="w-4 h-4" />
        <span className="flex-1 text-left">Search</span>
        <kbd className="hidden sm:inline-flex items-center gap-1 px-1.5 py-0.5 bg-[#0d1117] border border-[#30363d] rounded text-xs text-gray-400 font-mono">
          ⌘ K
        </kbd>
      </button>

      {isOpen && createPortal(
        <div className="fixed inset-0 z-100 flex items-start justify-center" onClick={handleClose}>
          <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" />
          <div
            className="relative flex flex-col gap-4 my-16 mx-auto p-3 bg-[#161b22] border border-[#30363d] rounded-lg shadow-2xl"
            style={{
              width: 'min(calc(100vw - 60px), 900px)',
              height: 'min-content',
              maxHeight: 'min(calc(100vh - 128px), 900px)',
            }}
            onClick={e => e.stopPropagation()}
          >
            <div className="flex items-center gap-3 px-3 py-3 border-b border-[#30363d] bg-[#161b22] rounded-t-lg">
              <Search className="w-5 h-5 text-gray-400" />
              <input
                ref={inputRef}
                type="text"
                value={query}
                onChange={e => setQuery(e.target.value)}
                placeholder="Search documentation..."
                className="flex-1 bg-transparent text-gray-200 placeholder-gray-500 outline-none text-base"
              />
              {isPending && (
                <div className="w-4 h-4 border-2 border-[#fd7e14] border-t-transparent rounded-full animate-spin" />
              )}
              <button
                type="button"
                onClick={handleClose}
                className="p-1 text-gray-400 hover:text-gray-200 transition-colors"
                aria-label="Close search"
              >
                <Close className="w-5 h-5" />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto min-h-0">
              {query
                ? (
                    results.length > 0
                      ? (
                          <div className="p-2">
                            {results.map((result, index) => (
                              <SearchResultItem
                                key={result.href}
                                itemRef={(el: HTMLButtonElement | null) => (resultItemRef.current[index] = el)}
                                category={result.category}
                                title={result.title}
                                excerpt={result.excerpt}
                                isSelected={index === selectedIndex}
                                onClick={() => handleResultClick(result.href)}
                                query={query}
                              />
                            ))}
                          </div>
                        )
                      : isPending
                        ? (
                            <div className="p-8 text-center text-gray-500 text-sm">
                              Searching...
                            </div>
                          )
                        : (
                            <div className="p-8 text-center text-gray-500 text-sm">
                              No results found for "
                              {query}
                              "
                            </div>
                          )
                  )
                : (
                    <div className="p-8 text-center text-gray-500 text-sm">
                      Start typing to search documentation...
                    </div>
                  )}
            </div>

            <div className="flex items-center gap-4 px-3 py-2 border-t border-[#30363d] bg-[#0d1117] text-xs text-gray-500 rounded-b-lg">
              <span className="flex items-center gap-1">
                <kbd className="px-1.5 py-0.5 bg-[#161b22] border border-[#30363d] rounded font-mono">↑</kbd>
                <kbd className="px-1.5 py-0.5 bg-[#161b22] border border-[#30363d] rounded font-mono">↓</kbd>
                to navigate
              </span>
              <span className="flex items-center gap-1">
                <kbd className="px-1.5 py-0.5 bg-[#161b22] border border-[#30363d] rounded font-mono">↵</kbd>
                to select
              </span>
              <span className="flex items-center gap-1">
                <kbd className="px-1.5 py-0.5 bg-[#161b22] border border-[#30363d] rounded font-mono">esc</kbd>
                to close
              </span>
            </div>
          </div>
        </div>,
        document.body,
      )}
    </>
  )
}

function SearchResultItem({
  itemRef,
  category,
  title,
  excerpt,
  isSelected,
  onClick,
  query,
}: {
  itemRef: (el: HTMLButtonElement | null) => void
  category: string
  title: string
  excerpt?: string
  isSelected: boolean
  onClick: () => void
  query: string
}) {
  const highlightText = (text: string, highlight: string) => {
    if (!highlight.trim())
      return text

    const escapedHighlight = escapeRegex(highlight)
    const parts = text.split(new RegExp(`(${escapedHighlight})`, 'gi'))
    let charPosition = 0

    return (
      <>
        {parts.filter(Boolean).map((part) => {
          const key = `${charPosition}-${part.slice(0, 10)}`
          charPosition += part.length

          return part.toLowerCase() === highlight.toLowerCase()
            ? (
                <mark key={key} className="bg-[#fd7e14]/30 text-white">
                  {part}
                </mark>
              )
            : (
                <span key={key}>{part}</span>
              )
        })}
      </>
    )
  }

  return (
    <button
      ref={itemRef}
      type="button"
      onClick={onClick}
      className={`w-full flex flex-col gap-1 px-3 py-2 rounded-md transition-colors group text-left ${
        isSelected
          ? 'bg-[#21262d] ring-1 ring-[#fd7e14]/50'
          : 'hover:bg-[#21262d]'
      }`}
    >
      <div className="flex items-center gap-2 text-sm">
        <span className="text-gray-400">
          <span className="text-[#fd7e14]">#</span>
          {' '}
          {category}
        </span>
        <span className="text-gray-400">›</span>
        <span className={`${isSelected ? 'text-white' : 'text-gray-200 group-hover:text-white'}`}>
          {highlightText(title, query)}
        </span>
      </div>
      {excerpt && (
        <p className="text-xs text-gray-400 line-clamp-2">
          {highlightText(excerpt, query)}
        </p>
      )}
    </button>
  )
}
