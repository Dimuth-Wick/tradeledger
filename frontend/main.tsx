import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
// Inter, bundled with the app (works offline): a font made for screens,
// with clear, same-width digits for prices and PnL.
import "@fontsource-variable/inter";
import { getTheme } from "./connection/backend";
import { applyCachedTheme, rememberTheme } from "./logic/theme";

// Last run's theme at once (no flash), then the saved one from Rust.
applyCachedTheme();
getTheme().then(rememberTheme, () => {});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
