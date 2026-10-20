import { describe, expect, it } from "vitest";
import { defaults, listAllows, migrate, setSiteThemed, siteOverride, SETTINGS_VERSION } from "../src/shared/settings";

describe("settings", () => {
  it("fills defaults and clamps bad values", () => {
    const s = migrate({ brightness: 999, sepia: -5, mode: "weird", blacklist: ["a.com", 3, "", " b.com "], font: { enabled: true } });
    expect(s.version).toBe(SETTINGS_VERSION);
    expect(s.brightness).toBe(150);
    expect(s.sepia).toBe(0);
    expect(s.mode).toBe("follow");
    expect(s.blacklist).toEqual(["a.com", "b.com"]);
    expect(s.font).toEqual({ enabled: true, family: "Inter" });
    expect(migrate(null)).toEqual(defaults());
  });

  it("migrates version 1", () => {
    const s = migrate({ version: 1, disabledFor: ["old.com"], dark: false });
    expect(s.blacklist).toEqual(["old.com"]);
    expect(s.mode).toBe("light");
  });

  it("keeps valid per-site overrides and speed dial only", () => {
    const s = migrate({
      sites: { "x.com": { mode: "off", background: "#112233", text: "nope", css: "a{}" }, "": { mode: "dark" } },
      newtab: { speedDial: [{ title: "A", url: "https://a.org" }, { title: "B", url: "javascript:alert(1)" }] },
    });
    expect(s.sites).toEqual({ "x.com": { mode: "off", background: "#112233", css: "a{}" } });
    expect(s.newtab.speedDial).toEqual([{ title: "A", url: "https://a.org" }]);
  });

  it("blacklist and whitelist", () => {
    let s = defaults();
    expect(listAllows(s, "https://a.com/")).toBe(true);
    s = setSiteThemed(s, "a.com", false);
    expect(s.blacklist).toEqual(["a.com"]);
    expect(listAllows(s, "https://www.a.com/x")).toBe(false);
    s = { ...setSiteThemed(s, "a.com", true), listMode: "whitelist" };
    expect(listAllows(s, "https://a.com/")).toBe(false);
    s = setSiteThemed(s, "a.com", true);
    expect(s.whitelist).toEqual(["a.com"]);
    expect(listAllows(s, "https://a.com/")).toBe(true);
  });

  it("finds per-site overrides", () => {
    const s = migrate({ sites: { "github.com": { mode: "light" }, "github.com/*/settings": { mode: "off" } } });
    expect(siteOverride(s, "https://github.com/me/settings")?.override.mode).toBe("off");
    expect(siteOverride(s, "https://github.com/")?.pattern).toBe("github.com");
    expect(siteOverride(s, "https://gitlab.com/")).toBeUndefined();
  });
});
