import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { FloatingBar } from "./windows/FloatingBar";
import "./styles/app.css";

const isFloatingBar =
  new URLSearchParams(window.location.search).get("window") === "floating";

createRoot(document.getElementById("root")!).render(
  <StrictMode>{isFloatingBar ? <FloatingBar /> : <App />}</StrictMode>
);
