#!/usr/bin/env bash
set -euo pipefail

declare -A parts
parts["3620093"]=xc7s15cpga196-1
parts["3622093"]=xc7s6ftgb196-1
parts["362c093"]=xc7a50tcsg324-1
parts["362d093"]=xc7a35tiftg256-1L
parts["362e093"]=xc7a15tcpg236-1
parts["362f093"]=xc7s50csga324-1
parts["3631093"]=xc7a100tiftg256-1L
parts["3632093"]=xc7a75tfgg484-1
parts["3636093"]=xc7a200tfbg484-1
parts["3647093"]=xc7k70tfbv676-1
parts["364c093"]=xc7k160tfbg484-1
parts["3722093"]=xc7z010iclg225-1L
parts["3723093"]=xc7z007sclg400-1
parts["3727093"]=xc7z020iclg484-1L
parts["3728093"]=xc7z014sclg484-1
parts["372c093"]=xc7z030ifbg484-2L
parts["373b093"]=xc7z015iclg485-1L
parts["373c093"]=xc7z012sclg485-1
parts["37c2093"]=xc7a25tcpg238-1
parts["37c3093"]=xc7a12ticsg325-1L
parts["37c4093"]=xc7s25ftgb196-1
parts["37c7093"]=xc7s100fgga676-1
parts["37c8093"]=xc7s75fgga484-1
parts["3823093"]=xcku035-fbva676-1-c
parts["3824093"]=xcku025-ffva1156-1-c
parts["4826093"]=xczu1cg-sbva484-1-e
parts["4a42093"]=xczu3cg-sbva484-1-e
parts["4a43093"]=xczu2cg-sbva484-1-e
parts["4a44093"]=xck24-ubva530-2LV-c
parts["4a46093"]=xczu5cg-fbvb900-1-e
parts["4a47093"]=xczu4cg-fbvb900-1-e
parts["4a49093"]=xck26-sfvc784-2LV-c
parts["4a5a093"]=xczu7cg-fbvb900-1-e
parts["4a5c093"]=xcu30-fbvb900-2-e
parts["4a62093"]=xcku5p-ffva676-1-e
parts["4a63093"]=xcku3p-ffva676-1-e
parts["4a64093"]=xcau25p-ffvb676-1-e
parts["4a65093"]=xcau20p-ffvb676-1-e
parts["4ac2093"]=xcau15p-ffvb676-1-e
parts["4ac4093"]=xcau10p-ffvb676-1-e
parts["4af2093"]=xczu3tcg-sfvc784-1-e
parts["4af6093"]=xcau7p-fcva289-1-e
parts["4b37093"]=xcu200-fsgd2104-2-e
parts["4b7d093"]=xcu55c-fsvh2892-2L-e

# these parts are in the generated list, but fail to make a bitstream:
# parts["4ad5093"]=xcu26-vsva1365-2L-e
# parts["4b71093"]=xcvu35p-fsvh2104-1-e
# parts["4b77093"]=xcu50-fsvh2104-2-e
# parts["4b79093"]=xcvu37p-fsvh2892-1-e

case $1 in
all-idcodes)
    printf "%s\n" "${!parts[@]}"
    ;;
all-devices)
    printf "%s\n" "${parts[@]}"
    ;;
*)
    echo "${parts[$1]}"
    ;;
esac
