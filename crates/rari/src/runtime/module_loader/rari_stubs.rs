pub const RARI_ROUTER_STUB: &str = r#"
export function useRouter() {
  return {
    pathname: '',
    params: {},
    searchParams: new URLSearchParams(),
    push() { return Promise.resolve(); },
    replace() { return Promise.resolve(); },
    back() {},
    forward() {},
    refresh() {},
    prefetch() { return Promise.resolve(); },
  };
}

export function usePathname() { return ''; }
export function useParams() { return {}; }
export function useSearchParams() { return new URLSearchParams(); }

export default { useRouter, usePathname, useParams, useSearchParams };
"#;

pub const RARI_REACT_DOM_STUB: &str = r#"
export function createPortal(children) { return children; }
export function flushSync(fn) { return fn(); }
export function hydrateRoot() { return { render() {}, unmount() {} }; }
export function createRoot() { return { render() {}, unmount() {} }; }
export function render() {}
export function hydrate() {}
export function unmountComponentAtNode() { return false; }
export function findDOMNode() { return null; }
export function unstable_batchedUpdates(fn) { return fn(); }
export function useFormStatus() { return { pending: false, data: null, method: null, action: null }; }
export function useFormState(action, initialState) { return [initialState, action, false]; }
export function preconnect() {}
export function prefetchDNS() {}
export function preinit() {}
export function preload() {}
export const version = '19.0.0';
export default {};
"#;

pub const RARI_DEFAULT_STUB: &str = r#"
export default {};
"#;
