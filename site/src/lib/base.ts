const raw = import.meta.env.BASE_URL

export const base: string = raw.endsWith('/') ? raw : `${raw}/`
