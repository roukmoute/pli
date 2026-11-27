import { render } from "@solidjs/web";
import App from "./App";

const root = document.getElementById("app");
if (!root) throw new Error("Point de montage introuvable.");
render(() => <App />, root);
