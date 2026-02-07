import Box from "@mui/material/Box";
import {Outlet, Navigate, useLocation} from "react-router-dom";
import {Sidebar} from "./Sidebar";
import {useStore, authStore, selectUser, selectAuthStatus} from "@/stores";
import {LoadingSpinner} from "@/components/primitives";
import {ANIMATION, ROUTES} from "@/constants";

function AppLayout() {
  const status = useStore(authStore.store, selectAuthStatus);
  const user = useStore(authStore.store, selectUser);
  const location = useLocation();

  if (status === "idle" || status === "loading") {
    return (
      <Box
        sx={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          minHeight: "100vh",
        }}
      >
        <LoadingSpinner label="Loading..." />
      </Box>
    );
  }

  if (!user) {
    return <Navigate to={ROUTES.LOGIN} state={{from: location}} replace />;
  }

  return (
    <Box sx={{display: "flex", minHeight: "100vh"}}>
      <Sidebar />

      <Box
        component="main"
        sx={{
          flexGrow: 1,
          transition: `margin-left ${ANIMATION.NORMAL}ms ease`,
          p: 4,
        }}
      >
        <Outlet />
      </Box>
    </Box>
  );
}

export {AppLayout};
