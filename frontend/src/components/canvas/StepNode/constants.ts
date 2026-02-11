import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import RepeatOutlined from '@mui/icons-material/RepeatOutlined'
import ForumOutlined from '@mui/icons-material/ForumOutlined'
import SettingsOutlined from '@mui/icons-material/SettingsOutlined'

const STEP_TYPE_ICONS: Record<string, typeof SettingsOutlined> = {
  single: SmartToyOutlined,
  for_each: RepeatOutlined,
  room: ForumOutlined,
}

const DEFAULT_STEP_TYPE_ICON = SettingsOutlined

export { STEP_TYPE_ICONS, DEFAULT_STEP_TYPE_ICON }
