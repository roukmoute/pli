import "./App.module.css";
import Admin from "./admin/Admin";
import Mail from "./session/Mail";

export default function App() {
  if (["/admin", "/admin/"].includes(window.location.pathname))
    return <Admin />;
  return <Mail />;
}
