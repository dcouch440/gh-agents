import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { PipelineRunPage } from './PipelineRunPage'

describe('PipelineRunPage', () => {
  it('renders pipeline run with ids from params', () => {
    render(
      <MemoryRouter initialEntries={['/pipelines/test-p/runs/test-r']}>
        <Routes>
          <Route path="/pipelines/:id/runs/:runId" element={<PipelineRunPage />} />
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('Pipeline test-p — Run test-r')).toBeInTheDocument()
  })

  it('displays pipeline and run ids from route params', () => {
    render(
      <MemoryRouter initialEntries={['/pipelines/p1/runs/r1']}>
        <Routes>
          <Route path="/pipelines/:id/runs/:runId" element={<PipelineRunPage />} />
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('Pipeline p1 — Run r1')).toBeInTheDocument()
  })
})
