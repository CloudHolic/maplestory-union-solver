import js from '@eslint/js'
import { defineConfig, globalIgnores } from 'eslint/config'
import { createTypeScriptImportResolver } from 'eslint-import-resolver-typescript'
import importX from 'eslint-plugin-import-x'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import globals from 'globals'
import tseslint from 'typescript-eslint'

export default defineConfig([
  globalIgnores(['dist', 'wasm-pkg']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite,
      importX.flatConfigs.recommended,
      importX.flatConfigs.typescript
    ],
    languageOptions: {
      globals: globals.browser
    },
    settings: {
      'import-x/resolver-next': [
        createTypeScriptImportResolver({
          alwaysTryTypes: true,
          project: './tsconfig.app.json'
        })
      ]
    },
    rules: {
      'import-x/order': ['error', {
        groups: [
          'builtin',
          'external',
          'internal',
          ['parent', 'sibling', 'index']
        ],
        pathGroups: [
          {
            pattern: '@solver/wasm',
            group: 'external',
            position: 'after'
          },
          {
            pattern: '@/**',
            group: 'internal'
          }
        ],
        pathGroupsExcludedImportTypes: ['builtin'],
        'newlines-between': 'always',
        alphabetize: {
          order: 'asc',
          caseInsensitive: true
        }
      }],
      'import-x/newline-after-import': 'error',
      'import-x/no-duplicates': 'error',
      'import-x/no-unresolved': ['error', {
        ignore: ['\\?worker$', '\\?url$', '\\?raw$']
      }],
      'import-x/default': 'off',
      'sort-imports': ['error', {
        ignoreCase: true,
        ignoreDeclarationSort: true,
        ignoreMemberSort: false
      }]
    }
  }
])