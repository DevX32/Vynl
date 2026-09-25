import { mount } from "svelte";
import App from "./App.svelte";
import "./assets/app.css";

document.addEventListener("contextmenu", (e) => e.preventDefault());

const app = mount(App, { target: document.getElementById("app")! });

export default app;
