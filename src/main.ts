import { createApp } from "vue";
import "./assets/main.css";
import App from "./App.vue";
import router from "./router";
import { i18n } from "./i18n";
import naive from "naive-ui";
import 'vfonts/Lato.css';
import 'vfonts/FiraCode.css';

const app = createApp(App);
app.use(router);
app.use(i18n);
app.use(naive);
app.mount("#app");
