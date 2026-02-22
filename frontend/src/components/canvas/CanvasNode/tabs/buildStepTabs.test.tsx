import { describe, it, expect, vi } from 'vitest'
import { buildStepTabs } from './buildStepTabs'
import { Archetype } from '../registry'
vi.mock('./ChatTab', () => ({ ChatTab: () => null }))
vi.mock('./LiveStreamTab', () => ({ LiveStreamTab: () => null }))
vi.mock('./AgentRosterTab', () => ({ AgentRosterTab: () => null }))
vi.mock('./RoomMembersTab', () => ({ RoomMembersTab: () => null }))
vi.mock('./DebugLogTab', () => ({ DebugLogTab: () => null }))
vi.mock('./NotesTab', () => ({ NotesTab: () => null }))

const baseParams = {
  stepId: 'step-1',
}

describe('buildStepTabs', () => {
  it('always includes chat tab first', () => {
    const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.BLANK })
    expect(tabs[0]!.id).toBe('chat')
  })

  it('includes notes and debug tabs for non-manager archetypes', () => {
    const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.BLANK })
    const ids = tabs.map((t) => t.id)
    expect(ids).toContain('notes')
    expect(ids).toContain('debug')
  })

  it('includes run results tab when includeLiveStream is true', () => {
    const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.BLANK, includeLiveStream: true })
    const ids = tabs.map((t) => t.id)
    expect(ids).toContain('live')
  })

  it('excludes run results tab when includeLiveStream is false', () => {
    const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.BLANK, includeLiveStream: false })
    const ids = tabs.map((t) => t.id)
    expect(ids).not.toContain('live')
  })

  describe('WORKFORCE archetype', () => {
    it('includes agents tab', () => {
      const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.WORKFORCE })
      const ids = tabs.map((t) => t.id)
      expect(ids).toContain('agents')
    })

    it('does not include members tab', () => {
      const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.WORKFORCE })
      const ids = tabs.map((t) => t.id)
      expect(ids).not.toContain('members')
    })
  })

  describe('ROOM archetype', () => {
    it('includes members tab', () => {
      const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.ROOM })
      const ids = tabs.map((t) => t.id)
      expect(ids).toContain('members')
    })

    it('does not include agents or documents tabs', () => {
      const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.ROOM })
      const ids = tabs.map((t) => t.id)
      expect(ids).not.toContain('agents')
      expect(ids).not.toContain('documents')
    })
  })

  describe('MANAGER archetype', () => {
    it('excludes notes tab (manager sits above the DAG)', () => {
      const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.MANAGER })
      const ids = tabs.map((t) => t.id)
      expect(ids).not.toContain('notes')
    })

    it('includes chat and debug tabs', () => {
      const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.MANAGER })
      const ids = tabs.map((t) => t.id)
      expect(ids).toContain('chat')
      expect(ids).toContain('debug')
    })
  })

  describe('BLANK archetype', () => {
    it('does not include agents, documents, or members tabs', () => {
      const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.BLANK })
      const ids = tabs.map((t) => t.id)
      expect(ids).not.toContain('agents')
      expect(ids).not.toContain('documents')
      expect(ids).not.toContain('members')
    })
  })

  it('tab order: chat > live? > archetype-specific > notes > debug', () => {
    const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.WORKFORCE, includeLiveStream: true })
    const ids = tabs.map((t) => t.id)
    expect(ids).toEqual(['chat', 'live', 'agents', 'notes', 'debug'])
  })

  it('every tab has an id, icon, tooltip, and content', () => {
    const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.WORKFORCE, includeLiveStream: true })
    for (const tab of tabs) {
      expect(tab.id).toBeTruthy()
      expect(tab.icon).toBeTruthy()
      expect(tab.tooltip).toBeTruthy()
      expect(tab.content).toBeDefined()
    }
  })
})
