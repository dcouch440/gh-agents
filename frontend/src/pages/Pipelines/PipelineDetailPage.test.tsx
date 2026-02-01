import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { PipelineDetailPage } from './PipelineDetailPage'

describe('PipelineDetailPage', () => {
  it('renders pipeline detail with id from params', () => {
    render(
      <MemoryRouter initialEntries={['/pipelines/test-pipeline-id']}>
        <Routes>
          <Route path="/pipelines/:id" element={<PipelineDetailPage />} />
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('Pipeline: test-pipeline-id')).toBeInTheDocument()
  })

  it('displays pipeline id from route params', () => {
    render(
      <MemoryRouter initialEntries={['/pipelines/pipeline-789']}>
        <Routes>
          <Route path="/pipelines/:id" element={<PipelineDetailPage />} />
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getByText('Pipeline: pipeline-789')).toBeInTheDocument()
  })
})
