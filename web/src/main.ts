import { mount } from "svelte";
import App from "./App.svelte";
import "./assets/app.css";
import "blobatar/motion.css";
import { applyStoredTheme } from "./lib/theme";

applyStoredTheme();

document.addEventListener("contextmenu", (e) => e.preventDefault());

const app = mount(App, { target: document.getElementById("app")! });

export default app;
