import '@testing-library/jest-dom'

// Optional: if you're using Tauri APIs, mock them here
import { vi } from 'vitest'
vi.mock('@tauri-apps/api', () => ({
  invoke: vi.fn().mockResolvedValue('Hello World from Tauri')
}))
