import type { LayoutEntry } from './app-types'
import { findCommonLayoutChainLength } from './navigation-utils'

export interface LayoutDiff {
  commonLayouts: LayoutEntry[]
  unmountLayouts: LayoutEntry[]
  mountLayouts: LayoutEntry[]
  updateLayouts: LayoutEntry[]
}

export interface LayoutInstance {
  entry: LayoutEntry
  component: React.ComponentType<any>
  props: any
  key: string
  ref: React.RefObject<any>
  mountedAt: number
  lastUpdated: number
}

export class LayoutManager {
  private layoutInstances: Map<string, LayoutInstance>

  constructor() {
    this.layoutInstances = new Map()
  }

  public computeLayoutDiff(
    currentChain: LayoutEntry[],
    targetChain: LayoutEntry[],
    currentProps?: Map<string, any>,
    targetProps?: Map<string, any>,
  ): LayoutDiff {
    const commonLength = findCommonLayoutChainLength(currentChain, targetChain)

    const commonLayouts = currentChain.slice(0, commonLength)
    const unmountLayouts = currentChain.slice(commonLength)
    const mountLayouts = targetChain.slice(commonLength)
    const updateLayouts: LayoutEntry[] = []

    if (currentProps && targetProps) {
      for (const layout of commonLayouts) {
        const currentLayoutProps = currentProps.get(layout.path)
        const targetLayoutProps = targetProps.get(layout.path)

        if (this.havePropsChanged(currentLayoutProps, targetLayoutProps)) {
          updateLayouts.push(layout)
        }
      }
    }

    return {
      commonLayouts,
      unmountLayouts,
      mountLayouts,
      updateLayouts,
    }
  }

  private havePropsChanged(currentProps: any, targetProps: any): boolean {
    if (currentProps == null && targetProps == null) {
      return false
    }

    if (currentProps == null || targetProps == null) {
      return true
    }

    if (typeof currentProps !== typeof targetProps) {
      return true
    }

    if (typeof currentProps !== 'object') {
      return currentProps !== targetProps
    }

    const currentKeys = Object.keys(currentProps)
    const targetKeys = Object.keys(targetProps)

    if (currentKeys.length !== targetKeys.length) {
      return true
    }

    for (const key of currentKeys) {
      if (key === 'children') {
        continue
      }

      if (!(key in targetProps)) {
        return true
      }

      if (typeof currentProps[key] === 'object' && typeof targetProps[key] === 'object') {
        if (this.havePropsChanged(currentProps[key], targetProps[key])) {
          return true
        }
      }
      else if (currentProps[key] !== targetProps[key]) {
        return true
      }
    }

    return false
  }

  public applyLayoutDiff(diff: LayoutDiff): void {
    for (const layout of diff.unmountLayouts) {
      this.unmountLayout(layout)
    }

    for (const layout of diff.mountLayouts) {
      this.mountLayout(layout)
    }

    for (const layout of diff.updateLayouts) {
      this.updateLayout(layout)
    }
  }

  public preserveLayout(entry: LayoutEntry): void {
    const instance = this.layoutInstances.get(entry.path)
    if (instance) {
      instance.lastUpdated = Date.now()
    }
  }

  public mountLayout(entry: LayoutEntry): void {
    const now = Date.now()

    const instance: LayoutInstance = {
      entry,
      component: null as any,
      props: {},
      key: `layout-${entry.path}-${now}`,
      ref: { current: null },
      mountedAt: now,
      lastUpdated: now,
    }

    this.layoutInstances.set(entry.path, instance)
  }

  public unmountLayout(entry: LayoutEntry): void {
    this.layoutInstances.delete(entry.path)
  }

  public updateLayout(entry: LayoutEntry, props?: any): void {
    const instance = this.layoutInstances.get(entry.path)
    if (instance) {
      if (props) {
        instance.props = props
      }
      instance.lastUpdated = Date.now()
    }
  }

  public getLayoutInstance(path: string): LayoutInstance | undefined {
    return this.layoutInstances.get(path)
  }

  public isLayoutMounted(path: string): boolean {
    return this.layoutInstances.has(path)
  }

  public getAllInstances(): LayoutInstance[] {
    return Array.from(this.layoutInstances.values())
  }

  public clearAll(): void {
    this.layoutInstances.clear()
  }

  public getInstanceCount(): number {
    return this.layoutInstances.size
  }
}
