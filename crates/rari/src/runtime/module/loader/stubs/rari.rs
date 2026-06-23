pub const RARI_ROUTER_STUB: &str = r"
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
";

pub const RARI_REACT_DOM_STUB: &str = r"
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
";

pub const RARI_HEADERS_STUB: &str = r"
export async function cookies() {
  return {
    get() { return undefined; },
    getAll() { return []; },
    set() {},
    delete() {},
    has() { return false; },
  };
}
export default { cookies };
";

pub const RARI_IMAGE_STUB: &str = r"
export function Image(props) {
  const { src, alt, width, height, className, style, ...rest } = props || {};
  return {
    type: 'img',
    props: { src: typeof src === 'object' ? src.src : src, alt: alt || '', width, height, className, style },
  };
}
export const DEFAULT_DEVICE_SIZES = [640, 750, 828, 1080, 1200, 1920, 2048, 3840];
export const DEFAULT_IMAGE_SIZES = [16, 32, 48, 64, 96, 128, 256, 384];
export const DEFAULT_FORMATS = ['image/webp'];
export const DEFAULT_QUALITY_LEVELS = { low: 25, medium: 50, high: 75, max: 100 };
export const DEFAULT_MAX_CACHE_SIZE = 50 * 1024 * 1024;
export const DEFAULT_MINIMUM_CACHE_TTL = 60;
export default Image;
";

pub const RARI_CLIENT_STUB: &str = r"
export function ClientRouter() { return null; }
export function NavigationErrorHandler() { return null; }
export function StatePreserver() { return null; }
export function clearPropsCache() {}
export function clearPropsCacheForComponent() {}
export function extractMetadata() { return {}; }
export function extractServerProps() { return {}; }
export function extractServerPropsWithCache() { return {}; }
export function extractStaticParams() { return []; }
export function hasServerSideDataFetching() { return false; }
export default {};
";

pub const RARI_DEFAULT_STUB: &str = r"
export default {};
";
