import { render, screen } from '@testing-library/react'
import { BrowserRouter, Route, Routes } from 'react-router-dom'
import { AppLayout } from './AppLayout'

describe('AppLayout', () => {
  it('renders sidebar and outlet', () => {
    render(
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<AppLayout />}>
            <Route index element={<div>Test Page Content</div>} />
          </Route>
        </Routes>
      </BrowserRouter>,
    )

    expect(screen.getByText('nexor')).toBeInTheDocument()
    expect(screen.getByText('Test Page Content')).toBeInTheDocument()
  })

  it('renders multiple routes through outlet', () => {
    render(
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<AppLayout />}>
            <Route index element={<div>Home</div>} />
            <Route path="about" element={<div>About</div>} />
          </Route>
        </Routes>
      </BrowserRouter>,
    )

    expect(screen.getByText('nexor')).toBeInTheDocument()
    expect(screen.getByText('Home')).toBeInTheDocument()
  })
})
