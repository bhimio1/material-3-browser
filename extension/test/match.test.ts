import { describe, expect, it } from "vitest";
import { bestMatch, matches, siteOf, urlKey } from "../src/shared/match";

describe("site patterns", () => {
  it("bare hosts match the site and its subdomains", () => {
    expect(matches("example.com", "https://example.com/")).toBe(true);
    expect(matches("example.com", "https://www.example.com/a?b=1")).toBe(true);
    expect(matches("example.com", "https://badexample.com/")).toBe(false);
    expect(matches("example.com", "https://example.com.evil.net/")).toBe(false);
  });

  it("wildcards", () => {
    expect(matches("*.google.com", "https://mail.google.com/x")).toBe(true);
    expect(matches("*.google.com", "https://google.com/")).toBe(true);
    expect(matches("google.*", "https://www.google.co.uk/search?q=1")).toBe(true);
    expect(matches("google.*", "https://google.com/")).toBe(true);
    expect(matches("google.*", "https://notgoogle.com/")).toBe(false);
    expect(matches("*", "https://anything.org/x")).toBe(true);
  });

  it("paths", () => {
    expect(matches("github.com/*/settings", "https://github.com/me/settings")).toBe(true);
    expect(matches("github.com/*/settings", "https://github.com/me/repo/settings")).toBe(true);
    expect(matches("github.com/*/settings", "https://github.com/me/issues")).toBe(false);
    expect(matches("example.com/docs", "https://example.com/docs/page")).toBe(true);
    expect(matches("example.com/docs", "https://example.com/docsx")).toBe(false);
  });

  it("regex and schemes", () => {
    expect(matches("/^news\\.[a-z]+\\.com\\//", "https://news.ycombinator.com/item")).toBe(true);
    expect(matches("https://example.com", "https://example.com/")).toBe(true);
    expect(matches("/[unclosed/", "https://x.com")).toBe(false);
    expect(urlKey("about:blank")).toBeNull();
    expect(urlKey("moz-extension://x/newtab.html")).toBeNull();
  });

  it("most specific match wins", () => {
    expect(bestMatch(["github.com", "github.com/*/settings"], "https://github.com/a/settings")).toBe("github.com/*/settings");
    expect(bestMatch(["x.com"], "https://y.com")).toBeUndefined();
  });

  it("site of a URL", () => {
    expect(siteOf("https://www.reddit.com/r/linux")).toBe("reddit.com");
    expect(siteOf("about:addons")).toBeNull();
  });
});
