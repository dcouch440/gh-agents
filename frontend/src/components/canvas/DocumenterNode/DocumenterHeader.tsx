import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import {useTheme} from "@mui/material/styles";
import {DocumenterIcon} from "@/animated/framer-motion-animation";

type DocumenterHeaderProps = {
  name: string;
  documentNames: string[];
  documentCount: number;
  modelId: string | null;
  agentName: string | null;
};

function DocumenterHeader({
  name,
  documentNames,
  documentCount,
  modelId,
  agentName,
}: DocumenterHeaderProps) {
  const theme = useTheme();
  const docSummary =
    documentNames.length > 0 ? documentNames.join(" \u00b7 ") : null;

  return (
    <Box
      sx={{
        width: "100%",
        height: "100%",
        display: "flex",
        alignItems: "center",
        gap: 1.5,
        px: 1.5,
        overflow: "hidden",
      }}
    >
      {/* Icon */}
      <Box
        sx={{
          flexShrink: 0,
          width: 36,
          height: 36,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <DocumenterIcon />
      </Box>

      {/* Title + subtitle */}
      <Box
        sx={{
          flex: 1,
          minWidth: 0,
          display: "flex",
          flexDirection: "column",
          gap: 0.25,
        }}
      >
        <Typography
          sx={{
            fontSize: 14,
            fontWeight: 600,
            color: "text.primary",
            lineHeight: 1.2,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {name}
        </Typography>
        <Typography
          sx={{
            fontSize: 11,
            color: docSummary !== null ? "text.secondary" : "text.disabled",
            lineHeight: 1.2,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {docSummary ?? "No documents"}
        </Typography>
      </Box>

      {/* Metadata badges */}
      <Box
        sx={{
          flexShrink: 0,
          display: "flex",
          alignItems: "center",
          gap: 0.5,
        }}
      >
        {documentCount > 0 && (
          <Box
            sx={{
              display: "inline-flex",
              alignItems: "center",
              px: 0.75,
              py: 0.25,
              borderRadius: "4px",
              backgroundColor: theme.palette.custom.hoverOverlay,
              border: 1,
              borderColor: "divider",
            }}
          >
            <Typography
              sx={{
                fontSize: 10,
                color: "text.secondary",
                lineHeight: 1,
                whiteSpace: "nowrap",
              }}
            >
              {documentCount} {documentCount === 1 ? "doc" : "docs"}
            </Typography>
          </Box>
        )}
        {agentName !== null && (
          <Box
            sx={{
              display: "inline-flex",
              alignItems: "center",
              px: 0.75,
              py: 0.25,
              borderRadius: "4px",
              backgroundColor: theme.palette.custom.hoverOverlay,
              border: 1,
              borderColor: "divider",
            }}
          >
            <Typography
              sx={{
                fontSize: 10,
                color: "text.secondary",
                lineHeight: 1,
                whiteSpace: "nowrap",
                maxWidth: 80,
                overflow: "hidden",
                textOverflow: "ellipsis",
              }}
            >
              {agentName}
            </Typography>
          </Box>
        )}
        {modelId !== null && (
          <Box
            sx={{
              display: "inline-flex",
              alignItems: "center",
              px: 0.75,
              py: 0.25,
              borderRadius: "4px",
              backgroundColor: theme.palette.custom.hoverOverlay,
              border: 1,
              borderColor: "divider",
            }}
          >
            <Typography
              sx={{
                fontSize: 10,
                color: "text.secondary",
                lineHeight: 1,
                whiteSpace: "nowrap",
                maxWidth: 80,
                overflow: "hidden",
                textOverflow: "ellipsis",
              }}
            >
              {modelId}
            </Typography>
          </Box>
        )}
      </Box>
    </Box>
  );
}

export {DocumenterHeader};
export type {DocumenterHeaderProps};
