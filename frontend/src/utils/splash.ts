const SPLASH_ID = 'nexor-splash'
const EXIT_CLASS = 'splash-exit'
const HIDDEN_CLASS = 'splash-hidden'

const dismissSplash = (): void => {
  const splash = document.getElementById(SPLASH_ID)
  if (!splash || splash.classList.contains(EXIT_CLASS)) return

  splash.classList.add(EXIT_CLASS)
  splash.addEventListener('transitionend', () => splash.classList.add(HIDDEN_CLASS), { once: true })

  // Safety fallback if transitionend never fires (e.g. reduced motion)
  setTimeout(() => splash.classList.add(HIDDEN_CLASS), 700)
}

export { dismissSplash }
