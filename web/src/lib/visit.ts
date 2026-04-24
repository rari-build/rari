interface ASTNode {
  type: string
  children?: ASTNode[]
  [key: string]: any
}

type Visitor = (node: ASTNode, index?: number, parent?: ASTNode) => void

export function visit(tree: ASTNode, visitor: Visitor): void {
  function walk(node: ASTNode, index?: number, parent?: ASTNode): void {
    visitor(node, index, parent)

    if (node.children && Array.isArray(node.children)) {
      for (let i = 0; i < node.children.length; i++) {
        walk(node.children[i], i, node)
      }
    }
  }

  walk(tree)
}
