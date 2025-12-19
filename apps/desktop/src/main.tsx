import React from "react";
import ReactDOM from "react-dom/client";
import { ChakraProvider, createSystem, defaultConfig } from "@chakra-ui/react";
import App from "./App";

const system = createSystem(defaultConfig);

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ChakraProvider value={system}>
      <App />
    </ChakraProvider>
  </React.StrictMode>
);
