/** Keep authored links independent of GitHub Pages' repository prefix. */
export default function baseLinks({ base = '/' } = {}) {
  return (tree) => {
    function walk(node) {
      if (typeof node.url === 'string' && node.url.startsWith('/') && !node.url.startsWith('//')) {
        node.url = `${base.replace(/\/$/, '')}${node.url}`;
      }
      for (const child of node.children ?? []) walk(child);
    }
    walk(tree);
  };
}
