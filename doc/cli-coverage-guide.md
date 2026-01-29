# CLI Coverage Guide

## Running Coverage

```bash
cd cli
npm install -D @vitest/coverage-v8   # one-time setup
npx vitest run --coverage            # run report
```

## Fulfilling a Coverage Ticket

1. **Run the coverage report** to identify gaps:
   ```bash
   npx vitest run --coverage
   ```
2. **Read the table** — look for files with low `% Stmts`, `% Lines`, or `% Branch`. Note the `Uncovered Line #s` column.
3. **Read the uncovered source lines** to understand what code paths are missing tests.
4. **Read existing test files** (`.test.ts`/`.test.tsx` siblings) to understand mocking patterns already in use.
5. **Write tests** targeting the uncovered lines. Common patterns in this project:
   - `vi.mock()` at the top for module-level mocks
   - `vi.mocked()` to get typed access to mocked functions
   - `ink-testing-library`'s `render()` + `lastFrame()` for component output assertions
   - `vi.waitFor()` for async state updates
   - For `ink-text-input`, mock the module and capture `onSubmit` to invoke it directly
6. **Re-run coverage** to confirm improvement.
7. **Commit** with format: `test(cli): <description>`

## Key Files

| Source | Test |
|---|---|
| `src/App.tsx` | `src/App.test.tsx` |
| `src/parseArgs.ts` | `src/parseArgs.test.ts` |
| `src/api/client.ts` | `src/api/client.test.ts` |
| `src/components/Login.tsx` | `src/components/Login.test.tsx` |
| `src/components/loginHandler.ts` | `src/components/loginHandler.test.ts` |
| `src/store/auth.ts` | `src/store/auth.test.ts` |
