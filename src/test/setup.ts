import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

// Each test installs its own invoke mock via mockIPC; ensure DOM + mocks reset.
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  // FirstRunWizard persists dismissal in localStorage; reset between tests.
  window.localStorage.clear();
});
