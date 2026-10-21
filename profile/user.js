// Material 3 Browser profile preferences.
// m3-browser rewrites this file on every start; put your own prefs in
// about:config or in user-overrides.js next to it (not read by Firefox,
// kept for your reference).

// ---- Material 3 look ----
// userChrome.css / userContent.css from the chrome folder.
user_pref("toolkit.legacyUserProfileCustomizations.stylesheets", true);
// Zen ignores WebExtension themes unless this is off; the extension paints
// the browser UI with your palette through the theme API.
user_pref("zen.theme.disable-lightweight", false);
user_pref("zen.theme.border-radius", 16);
user_pref("zen.theme.content-element-separation", 8);
user_pref("zen.welcome-screen.seen", true);
// Ctrl+T opens the Material 3 new tab page instead of Zen's floating URL bar.
user_pref("zen.urlbar.replace-newtab", false);
// No "Update complete" card on a fresh profile (Zen still updates normally).
user_pref("zen.updates.show-update-notification", false);
user_pref("svg.context-properties.content.enabled", true);
user_pref("layout.css.has-selector.enabled", true);
user_pref("widget.gtk.rounded-bottom-corners.enabled", true);

// ---- Add-ons ----
// Enable add-ons placed in the profile's extensions folder (uBlock Origin,
// and Material 3 Browser's own extension) without a confirmation prompt.
user_pref("extensions.autoDisableScopes", 0);
user_pref("extensions.enabledScopes", 15);
user_pref("extensions.pocket.enabled", false);
user_pref("extensions.getAddons.showPane", false);
user_pref("extensions.htmlaboutaddons.recommendations.enabled", false);

// ---- First run: no welcome tour, onboarding modal or what's-new pages ----
user_pref("browser.preonboarding.enabled", false);
user_pref("browser.aboutwelcome.enabled", false);
user_pref("datareporting.policy.dataSubmissionPolicyBypassNotification", true);
user_pref("startup.homepage_welcome_url", "");
user_pref("startup.homepage_welcome_url.additional", "");
user_pref("browser.startup.homepage_override.mstone", "ignore");
user_pref("browser.messaging-system.whatsNewPanel.enabled", false);
user_pref("browser.uitour.enabled", false);

// ---- Desktop integration ----
user_pref("widget.use-xdg-desktop-portal.file-picker", 1);
user_pref("widget.use-xdg-desktop-portal.mime-handler", 1);
user_pref("media.ffmpeg.vaapi.enabled", true);
user_pref("browser.shell.checkDefaultBrowser", false);
user_pref("browser.aboutConfig.showWarning", false);

// ---- New tab and home (the extension provides its own new tab) ----
user_pref("browser.newtabpage.activity-stream.showSponsored", false);
user_pref("browser.newtabpage.activity-stream.showSponsoredTopSites", false);
user_pref("browser.newtabpage.activity-stream.feeds.section.topstories", false);
user_pref("browser.newtabpage.activity-stream.feeds.snippets", false);
user_pref("browser.urlbar.suggest.quicksuggest.sponsored", false);
user_pref("browser.urlbar.suggest.quicksuggest.nonsponsored", false);

// ---- Privacy: no telemetry, studies or pings ----
user_pref("datareporting.healthreport.uploadEnabled", false);
user_pref("datareporting.policy.dataSubmissionEnabled", false);
user_pref("toolkit.telemetry.enabled", false);
user_pref("toolkit.telemetry.unified", false);
user_pref("toolkit.telemetry.archive.enabled", false);
user_pref("app.shield.optoutstudies.enabled", false);
user_pref("app.normandy.enabled", false);
user_pref("browser.ping-centre.telemetry", false);
user_pref("browser.discovery.enabled", false);
user_pref("browser.contentblocking.category", "strict");
user_pref("privacy.globalprivacycontrol.enabled", true);
