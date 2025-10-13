# Testing Guide for Meet Scribe

This document describes the testing strategy and implementation for Meet Scribe.

## Testing Stack

### Backend (Rust)
- **Test Framework**: Built-in Rust test framework (`#[test]`, `#[cfg(test)]`)
- **Mocking**: `mockall` crate for mock implementations
- **Async Testing**: `tokio-test` for async test utilities
- **Temp Files**: `tempfile` for temporary file management in tests
- **Coverage Tool**: `cargo-tarpaulin` (install with `cargo install cargo-tarpaulin`)

### Frontend (React/TypeScript)
- **Test Framework**: Vitest
- **Testing Library**: `@testing-library/react`
- **Mocking**: Vitest's built-in mocking capabilities
- **Coverage**: Vitest coverage reporter

## Running Tests

### Backend Tests

```bash
# Run all Rust tests
cd apps/desktop/src-tauri
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_keychain_operations

# Run tests with coverage
cargo tarpaulin --out Html --output-dir coverage
```

### Frontend Tests

```bash
# Run all frontend tests
cd apps/desktop
npm test

# Run tests in watch mode
npm run test:watch

# Run tests with coverage
npm run test:coverage
```

## Test Coverage Goals

- **Backend**: 100% coverage for core business logic
- **Frontend**: 80%+ coverage for React components
- **Integration**: Key user flows tested end-to-end

## Test Structure

### Backend Test Organization

```
src-tauri/src/
├── utils/
│   ├── keychain.rs
│   └── keychain_tests.rs          # Keychain unit tests
├── domain/
│   ├── models.rs
│   └── models_tests.rs            # Domain model tests
├── commands/
│   ├── config.rs
│   └── config_tests.rs            # Command tests (mocked)
├── adapters/
│   └── storage/
│       ├── sqlite.rs
│       └── sqlite_tests.rs        # SQLite adapter tests
└── ports/
    └── mocks.rs                    # Mock implementations for testing
```

### Frontend Test Organization

```
src/
├── pages/
│   ├── Settings.tsx
│   └── Settings.test.tsx           # Settings page tests
├── hooks/
│   └── useApiKeys.test.ts          # Custom hook tests
└── components/
    └── ServiceCard.test.tsx        # Component tests
```

## Test Implementation Status

### ✅ Completed

1. **Keychain Module Tests** (`keychain_tests.rs`)
   - ✅ Save and retrieve API keys
   - ✅ Delete API keys
   - ✅ Check key existence
   - ✅ Overwrite keys
   - ✅ Multiple providers
   - ✅ Special characters
   - ✅ Long keys
   - ✅ Empty keys
   - ✅ Nonexistent keys

2. **Mock Storage Implementation** (`ports/mocks.rs`)
   - ✅ In-memory storage for testing
   - ✅ All StoragePort methods implemented
   - ✅ Thread-safe with Arc<Mutex>
   - ✅ Supports all CRUD operations

### 🚧 In Progress

3. **Domain Model Tests**
   - Meeting creation and lifecycle
   - Participant management
   - Transcript handling
   - Insight generation
   - ServiceConfig validation

4. **Command Tests**
   - API key management commands
   - Service configuration commands
   - Service activation logic

5. **SQLite Adapter Tests**
   - Database CRUD operations
   - Migration testing
   - Transaction handling

### 📝 Pending

6. **Frontend Component Tests**
   - Settings page rendering
   - Form submission
   - Error handling
   - Loading states

7. **Integration Tests**
   - End-to-end user flows
   - Database + keychain integration
   - Tauri IPC communication

## Writing Tests

### Backend Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_meeting() {
        let meeting = Meeting::new(Platform::Teams, Some("Test Meeting".to_string()));

        assert!(meeting.id.is_none());
        assert_eq!(meeting.platform, Platform::Teams);
        assert_eq!(meeting.title, Some("Test Meeting".to_string()));
        assert!(meeting.end_time.is_none());
    }

    #[tokio::test]
    async fn test_async_operation() {
        let storage = MockStorage::new();
        let meeting = Meeting::new(Platform::Zoom, None);

        let id = storage.create_meeting(&meeting).await.unwrap();
        assert!(id > 0);

        let retrieved = storage.get_meeting(id).await.unwrap();
        assert!(retrieved.is_some());
    }
}
```

### Frontend Test Example

```typescript
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import Settings from './Settings';

describe('Settings Page', () => {
  it('renders API key inputs', () => {
    render(<Settings />);
    expect(screen.getByPlaceholderText(/enter api key/i)).toBeInTheDocument();
  });

  it('saves API key when form is submitted', async () => {
    const mockInvoke = vi.fn().mockResolvedValue({});
    window.__TAURI__ = { invoke: mockInvoke };

    render(<Settings />);
    const input = screen.getByPlaceholderText(/enter api key/i);
    const button = screen.getByRole('button', { name: /save/i });

    fireEvent.change(input, { target: { value: 'test-api-key' } });
    fireEvent.click(button);

    expect(mockInvoke).toHaveBeenCalledWith('save_api_key', {
      request: {
        service_type: 'asr',
        provider: 'deepgram',
        api_key: 'test-api-key',
      },
    });
  });
});
```

## Mocking Strategies

### Mocking Tauri Commands

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::mocks::MockStorage;

    #[tokio::test]
    async fn test_save_api_key_command() {
        let storage = Arc::new(MockStorage::new());
        let keychain = Arc::new(KeychainManager::new());

        let state = AppState { storage, keychain };

        let request = SaveApiKeyRequest {
            service_type: "asr".to_string(),
            provider: "deepgram".to_string(),
            api_key: "test-key".to_string(),
        };

        let result = save_api_key(tauri::State::from(&state), request).await;
        assert!(result.is_ok());
    }
}
```

### Mocking Database

```rust
#[tokio::test]
async fn test_with_mock_storage() {
    let mock_storage = MockStorage::new();

    // Setup test data
    let meeting = Meeting::new(Platform::Meet, Some("Test".to_string()));
    let id = mock_storage.create_meeting(&meeting).await.unwrap();

    // Test operations
    let retrieved = mock_storage.get_meeting(id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().title, Some("Test".to_string()));
}
```

## Continuous Integration

### GitHub Actions Workflow

```yaml
name: Tests

on: [push, pull_request]

jobs:
  backend-tests:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      - name: Run tests
        run: |
          cd apps/desktop/src-tauri
          cargo test --verbose
      - name: Generate coverage
        run: cargo tarpaulin --out Xml
      - name: Upload coverage
        uses: codecov/codecov-action@v3

  frontend-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: '20'
      - name: Install dependencies
        run: |
          cd apps/desktop
          npm ci
      - name: Run tests
        run: npm test -- --coverage
```

## Test Data Management

### Test Fixtures

```rust
pub fn create_test_meeting() -> Meeting {
    Meeting {
        id: None,
        platform: Platform::Teams,
        title: Some("Test Meeting".to_string()),
        start_time: chrono::Utc::now().timestamp(),
        end_time: None,
        participant_count: Some(5),
        created_at: chrono::Utc::now().timestamp(),
    }
}
```

### Cleanup

```rust
impl Drop for TestContext {
    fn drop(&mut self) {
        // Clean up test keychain entries
        let _ = self.keychain.delete_api_key("test", "provider");
    }
}
```

## Coverage Reporting

### Generate HTML Coverage Report

```bash
# Backend
cd apps/desktop/src-tauri
cargo tarpaulin --out Html --output-dir ../../coverage/backend

# Frontend
cd apps/desktop
npm run test:coverage
# Report in coverage/frontend/index.html
```

### View Coverage

```bash
# Backend
open apps/desktop/coverage/backend/index.html  # macOS
xdg-open apps/desktop/coverage/backend/index.html  # Linux
start apps/desktop/coverage/backend/index.html  # Windows
```

## Best Practices

1. **Test Isolation**: Each test should be independent
2. **Clear Names**: Test names should describe what they test
3. **Arrange-Act-Assert**: Follow AAA pattern
4. **Mock External Dependencies**: Don't rely on real services
5. **Test Edge Cases**: Empty inputs, null values, errors
6. **Cleanup**: Always clean up test data (keychain, files)
7. **Fast Tests**: Keep tests fast for quick feedback
8. **Readable Assertions**: Use descriptive assertion messages

## Troubleshooting

### Tests Failing on CI But Passing Locally

- Check for environment-specific issues (paths, line endings)
- Ensure proper cleanup between tests
- Look for race conditions in async tests

### Keychain Tests Failing

- Keychain tests require OS-level permissions
- May need to run with specific user context
- Consider skip conditions for CI environments

### Coverage Not 100%

- Check for unreachable code
- Verify all branches are tested
- Look for platform-specific code that's not tested

## Resources

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Vitest Documentation](https://vitest.dev/)
- [React Testing Library](https://testing-library.com/react)
- [Cargo Tarpaulin](https://github.com/xd009642/tarpaulin)
