import { describe, it, expect, vi } from 'vitest'
import { buildStepTabs } from './buildStepTabs'
import { Archetype } from './archetypes'
vi.mock('./tabs/ChatTab', () => ({ ChatTab: () => null }))
vi.mock('./tabs/LiveStreamTab', () => ({ LiveStreamTab: () => null }))
vi.mock('./tabs/InputsOutputsTab', () => ({ InputsOutputsTab: () => null }))
vi.mock('./tabs/AgentRosterTab', () => ({ AgentRosterTab: () => null }))
vi.mock('./tabs/RoomMembersTab', () => ({ RoomMembersTab: () => null }))
vi.mock('./tabs/DebugLogTab', () => ({ DebugLogTab: () => null }))
vi.mock('./tabs/LastRunTab', () => ({ LastRunTab: () => null }))

const baseParams = {
  stepId: 'step-1',
  upstreamStepNames: ['Upstream A'] as readonly string[],
}

describe('buildStepTabs', () => {
  it('always includes chat tab first', () => {
    const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.BLANK })
    expect(tabs[0]!.id).toBe('chat')
  })

  it('always includes io, lastrun, and debug tabs', () => {
    const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.BLANK })
    const ids = tabs.map((t) => t.id)
    expect(ids).toContain('io')
    expect(ids).toContain('lastrun')
    expect(ids).toContain('debug')
  })

  it('includes live stream tab when includeLiveStream is true', () => {
    const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.BLANK, includeLiveStream: true })
    const ids = tabs.map((t) => t.id)
    expect(ids).toContain('live')
  })

  it('excludes live stream tab when includeLiveStream is false', () => {
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

  describe('BLANK archetype', () => {
    it('does not include agents, documents, or members tabs', () => {
      const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.BLANK })
      const ids = tabs.map((t) => t.id)
      expect(ids).not.toContain('agents')
      expect(ids).not.toContain('documents')
      expect(ids).not.toContain('members')
    })
  })

  it('tab order: chat > live? > io > archetype-specific > lastrun > debug', () => {
    const tabs = buildStepTabs({ ...baseParams, archetype: Archetype.WORKFORCE, includeLiveStream: true })
    const ids = tabs.map((t) => t.id)
    expect(ids).toEqual(['chat', 'live', 'io', 'agents', 'lastrun', 'debug'])
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
