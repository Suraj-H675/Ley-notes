import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource-variable/geist/wght.css";
import "@fontsource-variable/geist-mono/wght.css";
import "./shared/styles/index.css";
import { LegacyRecoveryPage } from "./legacy-recovery/LegacyRecoveryPage";

document.documentElement.dataset.surface = "website";

const rootElement = document.getElementById("root");
if (!rootElement) throw new Error("No #root element found in legacy-recovery.html");

createRoot(rootElement).render(
  <StrictMode>
    <LegacyRecoveryPage />
  </StrictMode>,
);
