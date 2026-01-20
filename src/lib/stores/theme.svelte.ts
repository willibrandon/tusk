import { browser } from '$app/environment';

type ThemeMode = 'light' | 'dark';

interface ThemeState {
	mode: ThemeMode;
	preferSystem: boolean;
}

function createThemeStore() {
	let mode = $state<ThemeMode>('light');
	let preferSystem = $state<boolean>(true);

	// Getter to read current preferSystem value (avoids state_referenced_locally warning)
	const isSystemPreferred = () => preferSystem;

	// Initialize from localStorage and system preference
	if (browser) {
		const stored = localStorage.getItem('theme');
		if (stored) {
			try {
				const parsed = JSON.parse(stored) as ThemeState;
				mode = parsed.mode;
				preferSystem = parsed.preferSystem;
			} catch {
				// Use defaults if parsing fails
			}
		}

		// Apply system preference if enabled
		if (isSystemPreferred()) {
			const systemDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
			mode = systemDark ? 'dark' : 'light';
		}

		// Listen for system preference changes
		window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
			if (isSystemPreferred()) {
				mode = e.matches ? 'dark' : 'light';
				applyTheme();
			}
		});

		// Apply initial theme
		applyTheme();
	}

	function applyTheme() {
		if (!browser) return;

		const root = document.documentElement;
		if (mode === 'dark') {
			root.classList.add('dark');
		} else {
			root.classList.remove('dark');
		}

		// Persist to localStorage
		localStorage.setItem('theme', JSON.stringify({ mode, preferSystem }));
	}

	function setMode(newMode: ThemeMode) {
		mode = newMode;
		preferSystem = false;
		applyTheme();
	}

	function toggle() {
		mode = mode === 'light' ? 'dark' : 'light';
		preferSystem = false;
		applyTheme();
	}

	function useSystemPreference() {
		preferSystem = true;
		if (browser) {
			const systemDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
			mode = systemDark ? 'dark' : 'light';
			applyTheme();
		}
	}

	return {
		get mode() {
			return mode;
		},
		get preferSystem() {
			return preferSystem;
		},
		setMode,
		toggle,
		useSystemPreference
	};
}

export const theme = createThemeStore();
