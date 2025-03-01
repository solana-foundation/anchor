"use client";
declare global {
  interface Window {
    Inkeep: any;
  }
}

import Script from "next/script";
import { settings } from "../inkeep-settings";

export function InkeepScript() {
  return (
    <Script
      id="inkeep-script"
      src="https://unpkg.com/@inkeep/uikit-js@0.3.14/dist/embed.js"
      type="module"
      strategy="afterInteractive"
      onReady={() => {
        const config = {
          colorModeSync: {
            observedElement: document.documentElement,
            isDarkModeCallback: (el: any) => {
              return el.classList.contains("dark");
            },
            colorModeAttribute: "class",
          },
          properties: settings,
        };
        window.Inkeep().embed({
          componentType: "ChatButton",
          ...config,
        });
      }}
    />
  );
}
