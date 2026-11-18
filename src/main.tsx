import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { getTheme } from "./lib/api";
import { applyCachedTheme, rememberTheme } from "./lib/theme";

// Last run's theme at once (no flash), then the saved one from Rust.
applyCachedTheme();
getTheme().then(rememberTheme, () => {});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
