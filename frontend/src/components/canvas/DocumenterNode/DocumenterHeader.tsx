import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import {useTheme} from "@mui/material/styles";
import {HeroIllustration} from "@/animated/framer-motion-animation";

type DocumenterHeaderProps = {
  name: string;
  documentNames: string[];
};

function DocumenterHeader({name, documentNames}: DocumenterHeaderProps) {
  const theme = useTheme();
  const docSummary =
    documentNames.length > 0 ? documentNames.join(" \u00b7 ") : null;

  return (
    <Box
      sx={{
        width: "100%",
        height: "100%",
        display: "flex",
        alignItems: "stretch",
        overflow: "hidden",
      }}
    >
      {/* SVG panel — its own background strip */}
      <Box
        sx={{
          flexShrink: 0,
          width: 80,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          overflow: "hidden",
          borderRight: 1,
          borderColor: "divider",
          backgroundColor: theme.palette.mode === "light"
            ? "rgba(255, 150, 79, 0.06)"
            : "rgba(59, 130, 246, 0.04)",
          "& > div": {width: "100%", height: "100%"},
          "& svg": {width: "160%", height: "160%", marginLeft: "-30%"},
        }}
      >
        <HeroIllustration />
      </Box>

      {/* Text panel */}
      <Box
        sx={{
          flex: 1,
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          px: 2,
          gap: 0.25,
          minWidth: 0,
        }}
      >
        <Typography
          sx={{
            fontSize: 16,
            fontWeight: 600,
            color: "text.primary",
            lineHeight: 1.2,
          }}
        >
          {name}
        </Typography>
        {docSummary !== null && (
          <Typography
            sx={{
              fontSize: 10,
              color: "text.secondary",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {docSummary}
          </Typography>
        )}
        {documentNames.length > 0 && (
          <Typography
            sx={{
              fontSize: 9,
              color: "text.disabled",
            }}
          >
            {documentNames.length}{" "}
            {documentNames.length === 1 ? "document" : "documents"}
          </Typography>
        )}
      </Box>
    </Box>
  );
}

export {DocumenterHeader};
export type {DocumenterHeaderProps};
