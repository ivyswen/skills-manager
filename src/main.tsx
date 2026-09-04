import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// 禁用默认浏览器右键上下文菜单
window.addEventListener("contextmenu", (e) => {
  e.preventDefault();
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <App />
);
