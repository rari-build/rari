'use client'

import React from 'react'

interface LayoutWrapperProps {
  children: React.ReactNode
  layoutPath: string
  layoutKey: string
}

export function LayoutWrapper({ children }: LayoutWrapperProps) {
  return <>{children}</>
}
