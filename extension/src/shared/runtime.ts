/**
 * The browser's own runtime.sendMessage, saved before Dark Reader loads.
 * Dark Reader's API build wraps chrome.runtime.sendMessage (which is
 * browser.runtime.sendMessage in Firefox) and drops its return value, so a
 * content script that imports it never sees the background's replies.
 * Import this module before "darkreader".
 */
export const sendMessage: (message: unknown) => Promise<unknown> = browser.runtime.sendMessage.bind(browser.runtime);
