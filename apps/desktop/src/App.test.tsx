// src/App.test.tsx
import { render, screen } from "@testing-library/react";
import App from "./App";

describe("App", () => {
  it("renders sidebar title and nav links", () => {
    render(<App />); // App already includes BrowserRouter

    // Only the sidebar title is an h1; the dashboard line is a <p>
    expect(
      screen.getByRole("heading", { name: /meet scribe/i, level: 1 })
    ).toBeInTheDocument();

    expect(screen.getByRole("link", { name: /dashboard/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /active meeting/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /history/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /settings/i })).toBeInTheDocument();
  });
});
