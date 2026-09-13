/** Resolve a public path under either repository Pages or the custom domain. */
export const url = (path = '') => `${import.meta.env.BASE_URL.replace(/\/$/, '')}/${path.replace(/^\//, '')}`;
