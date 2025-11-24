'use client'

import React, { useEffect, useLayoutEffect, useRef } from 'react'

interface LayoutWrapperProps {
  children: React.ReactNode
  layoutPath: string
  layoutKey: string
}

export function LayoutWrapper({ children, layoutPath, layoutKey }: LayoutWrapperProps) {
  const mountTimeRef = useRef<number>(0)
  const renderCountRef = useRef<number>(0)

  useLayoutEffect(() => {
    if (mountTimeRef.current === 0) {
      mountTimeRef.current = Date.now()
    }
  }, [])

  useEffect(() => {
    renderCountRef.current += 1

    console.warn('[LayoutWrapper] Layout mounted/updated:', {
      layoutPath,
      layoutKey,
      mountTime: mountTimeRef.current,
      renderCount: renderCountRef.current,
    })

    return () => {
      console.warn('[LayoutWrapper] Layout unmounting:', {
        layoutPath,
        layoutKey,
        lifespan: Date.now() - mountTimeRef.current,
        totalRenders: renderCountRef.current,
      })
    }
  }, [layoutPath, layoutKey])

  return <>{children}</>
}
