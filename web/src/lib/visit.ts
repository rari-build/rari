interface ASTNode {
  type: string
  children?: ASTNode[]
  [key: string]: any
}

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- visitor mutates AST nodes in place
type Visitor = (node: ASTNode, index?: number, parent?: ASTNode) => void

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- tree is walked and mutated in place by visitors
export function visit(tree: ASTNode, visitor: Visitor): void {
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- node is mutated in place by visitors
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
