/**
 * Site patterns for the blacklist, whitelist and per-site overrides (and
 * Dark Reader's fix list):
 *
 *   example.com            the site and its subdomains
 *   *.example.com          the same (subdomains optional)
 *   google.*               any top-level domain
 *   github.com/*\/settings  a path; `*` in the path matches anything
 *   /regex/                a regular expression against host + path
 */

const cache = new Map<string, RegExp | null>();

function escape(s: string): string {
  return s.replace(/[.+?^${}()|[\]\\]/g, "\\$&");
}

export function patternToRegex(pattern: string): RegExp | null {
  const cached = cache.get(pattern);
  if (cached !== undefined) return cached;
  let re: RegExp | null = null;
  const p = pattern.trim().toLowerCase();
  try {
    if (p.length > 2 && p.startsWith("/") && p.endsWith("/")) {
      re = new RegExp(p.slice(1, -1));
    } else if (p) {
      const noScheme = p.replace(/^[a-z]+:\/\//, "");
      const slash = noScheme.indexOf("/");
      let host = slash < 0 ? noScheme : noScheme.slice(0, slash);
      const path = slash < 0 ? "" : noScheme.slice(slash);
      let hostRe: string;
      if (host === "*") {
        hostRe = "[^/]*";
      } else {
        let prefix = "(?:[^/]*\\.)?";
        if (host.startsWith("*.")) host = host.slice(2);
        let suffix = "";
        if (host.endsWith(".*")) {
          host = host.slice(0, -2);
          suffix = "\\.[^/]+";
        }
        hostRe = prefix + host.split("*").map(escape).join("[^/]*") + suffix;
        prefix = "";
      }
      let pathRe = "(?:[/?#].*)?";
      if (path && path !== "/" && path !== "/*") {
        pathRe = path.split("*").map(escape).join(".*") + (path.endsWith("*") ? "" : "(?:[/?#].*)?");
      }
      re = new RegExp(`^${hostRe}${pathRe}$`);
    }
  } catch {
    re = null;
  }
  cache.set(pattern, re);
  return re;
}

/** host + path of a URL, lowercased, without scheme, port or credentials. */
export function urlKey(url: string): string | null {
  try {
    const u = new URL(url);
    if (!/^(https?|file):$/.test(u.protocol)) return null;
    return (u.hostname + u.pathname + u.search).toLowerCase();
  } catch {
    return null;
  }
}

export function matches(pattern: string, url: string): boolean {
  const key = urlKey(url);
  const re = patternToRegex(pattern);
  return key !== null && re !== null && re.test(key);
}

export function matchesAny(patterns: string[], url: string): boolean {
  return patterns.some((p) => matches(p, url));
}

/** The most specific (longest) pattern that matches. */
export function bestMatch(patterns: string[], url: string): string | undefined {
  return patterns.filter((p) => matches(p, url)).sort((a, b) => b.length - a.length)[0];
}

/** A pattern for "this site": the host without a leading www. */
export function siteOf(url: string): string | null {
  try {
    const u = new URL(url);
    if (!/^https?:$/.test(u.protocol)) return null;
    return u.hostname.replace(/^www\./, "");
  } catch {
    return null;
  }
}
