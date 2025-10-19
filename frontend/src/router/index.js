import { createRouter, createWebHistory } from "vue-router";
import ProfileAnalyzer from "../views/ProfileAnalyzer.vue";

const routes = [
  {
    path: "/",
    name: "ProfileAnalyzer",
    component: ProfileAnalyzer,
  },
  {
    path: "/:pathMatch(.*)*",
    redirect: "/",
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
